use futures::future::try_join3;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex as AMutex;
use tokio::time::Duration;
use std::path::Path;
use std::pin::Pin;
use std::process::Output;
use std::sync::Arc;
use std::time::Instant;
use std::process::Stdio;
use tracing::error;


pub async fn write_to_stdin_and_flush(stdin: &mut ChildStdin, text_to_write: &str) -> Result<(), String>
{
    stdin.write_all(format!("{}\n", text_to_write).as_bytes()).await.map_err(|e| {
        error!("Failed to write to pdb stdin: {}", e);
        e.to_string()
    })?;
    stdin.flush().await.map_err(|e| {
        error!("Failed to flush pdb stdin: {}", e);
        e.to_string()
    })?;

    Ok(())
}

pub async fn blocking_read_until_token_or_timeout<
    StdoutReader: AsyncRead + Unpin,
    StderrReader: AsyncRead + Unpin,
>(
    stdout: &mut StdoutReader,
    stderr: &mut StderrReader,
    timeout_ms: u64,
    output_token: &str,
) -> Result<(String, String, bool), String> {
    assert!(timeout_ms > 0, "Timeout in ms must be positive to prevent indefinite reading if the stream lacks an EOF");
    let start_time = Instant::now();
    let timeout_duration = Duration::from_millis(timeout_ms);
    let mut output = Vec::new();
    let mut error = Vec::new();
    let mut output_buf = [0u8; 1024];
    let mut error_buf = [0u8; 1024];
    let mut have_the_token = false;

    while start_time.elapsed() < timeout_duration {
        let mut output_bytes_read = 0;
        let mut error_bytes_read = 0;
        tokio::select! {
            stdout_result = stdout.read(&mut output_buf) => {
                match stdout_result {
                    Ok(0) => {},
                    Ok(bytes_read) => {
                        output.extend_from_slice(&output_buf[..bytes_read]);
                        if !output_token.is_empty() && output.trim_ascii_end().ends_with(output_token.as_bytes()) {
                            have_the_token = true;
                        }
                        output_bytes_read = bytes_read;
                    },
                    Err(e) => return Err(format!("Error reading from stdout: {}", e)),
                }
            },
            stderr_result = stderr.read(&mut error_buf) => {
                match stderr_result {
                    Ok(0) => {},
                    Ok(bytes_read) => {
                        error.extend_from_slice(&error_buf[..bytes_read]);
                        error_bytes_read = bytes_read;
                    },
                    Err(e) => return Err(format!("Error reading from stderr: {}", e)),
                }
            },
            _ = tokio::time::sleep(Duration::from_millis(50)) => {},
        }
        if have_the_token && output_bytes_read == 0 && error_bytes_read == 0 { break; }
    }

    Ok((String::from_utf8_lossy(&output).to_string(), String::from_utf8_lossy(&error).to_string(), have_the_token))
}

pub async fn read_file_with_cursor(
    file_path: &Path,
    cursor: Arc<AMutex<u64>>,
) -> Result<(String, usize), String> {
    let file = tokio::fs::OpenOptions::new().read(true).open(file_path).await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let mut cursor_locked = cursor.lock().await;
    let mut file = tokio::io::BufReader::new(file);
    file.seek(tokio::io::SeekFrom::Start(*cursor_locked)).await
        .map_err(|e| format!("Failed to seek: {}", e))?;
    let mut buffer = String::new();
    let bytes_read = file.read_to_string(&mut buffer).await
        .map_err(|e| format!("Failed to read to buffer: {}", e))?;
    if bytes_read > 0 {
        *cursor_locked += bytes_read as u64;
    }
    Ok((buffer, bytes_read))
}

pub async fn is_someone_listening_on_that_tcp_port(port: u16, timeout: tokio::time::Duration) -> bool {
    match tokio::time::timeout(timeout, TcpStream::connect(&format!("127.0.0.1:{}", port))).await {
        Ok(Ok(_)) => true,    // Connection successful
        Ok(Err(_)) => false,  // Connection failed, refused
        Err(e) => {  // Timeout occurred
            tracing::error!("Timeout occurred while checking port {}: {}", port, e);
            false             // still no one is listening, as far as we can tell
        }
    }
}

pub fn first_n_chars(msg: &str, n: usize) -> String {
    let mut last_n_chars: String = msg.chars().take(n).collect();
    if last_n_chars.len() == n {
        last_n_chars.push_str("...");
    }
    return last_n_chars;
}

pub fn last_n_chars(msg: &str, n: usize) -> String {
    let mut last_n_chars: String = msg.chars().rev().take(n).collect::<String>().chars().rev().collect();
    if last_n_chars.len() == n {
        last_n_chars.insert_str(0, "...");
    }
    return last_n_chars;
}

pub fn last_n_lines(msg: &str, n: usize) -> String {
    let lines: Vec<&str> = msg.lines().filter(|line| !line.trim().is_empty()).collect();
    let start = if lines.len() > n { lines.len() - n } else { 0 };

    let mut output = if start > 0 { "...\n" } else { "" }.to_string();
    output.push_str(&lines[start..].join("\n"));
    output.push('\n');

    output
}

/// Reimplemented .wait_with_output() from tokio::process::Child to accept &mut self instead of self
/// Suggested by others with this problem: https://github.com/tokio-rs/tokio/issues/7138
fn wait_with_output<'a>(child: &'a mut Child) -> Pin<Box<dyn futures::Future<Output = Result<Output, futures::io::Error>> + Send + 'a>>
{
    Box::pin(async move {
        async fn read_to_end<A: AsyncRead + Unpin>(io: &mut Option<A>) -> Result<Vec<u8>, futures::io::Error> {
            let mut vec = Vec::new();
            if let Some(io) = io.as_mut() {
                io.read_to_end(&mut vec).await?;
            }
            Ok(vec)
        }

        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();

        let stdout_fut = read_to_end(&mut stdout_pipe);
        let stderr_fut = read_to_end(&mut stderr_pipe);

        let (status, stdout, stderr) =
            try_join3(child.wait(), stdout_fut, stderr_fut).await?;

        // Drop happens after `try_join` due to <https://github.com/tokio-rs/tokio/issues/4309>
        drop(stdout_pipe);
        drop(stderr_pipe);

        Ok(Output {
            status,
            stdout,
            stderr,
        })
    })
}

struct ChildWithKillOnDrop(Box<dyn process_wrap::tokio::TokioChildWrapper>);
impl Drop for ChildWithKillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.start_kill();
    }
}

pub async fn execute_command(mut cmd: Command, timeout_secs: u64, cmd_str: &str) -> Result<Output, String> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut cmd = process_wrap::tokio::TokioCommandWrap::from(cmd);
    #[cfg(unix)]
    cmd.wrap(process_wrap::tokio::ProcessGroup::leader());
    #[cfg(windows)]
    cmd.wrap(process_wrap::tokio::JobObject);

    let child = cmd.spawn()
        .map_err(|e| format!("command '{cmd_str}' failed to spawn: {e}"))?;
    let mut child = ChildWithKillOnDrop(child);

    tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_secs),
        wait_with_output(child.0.inner_mut())
    ).await
        .map_err(|_| format!("command '{cmd_str}' timed out after {timeout_secs} seconds"))?
        .map_err(|e| format!("command '{cmd_str}' failed to execute: {e}"))
}
