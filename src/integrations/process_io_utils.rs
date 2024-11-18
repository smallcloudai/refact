use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStdin};
use tokio::time::Duration;
use std::time::Instant;
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
        let mut error_bytes_read = 0;
        tokio::select! {
            stdout_result = tokio::time::timeout(Duration::from_millis(50), stdout.read(&mut output_buf)) => {
                match stdout_result {
                    Ok(Ok(0)) | Err(_) => {},
                    Ok(Ok(bytes_read)) => {
                        output.extend_from_slice(&output_buf[..bytes_read]);
                        if !output_token.is_empty() && output.trim_ascii_end().ends_with(output_token.as_bytes()) {
                            have_the_token = true;
                        }
                    },
                    Ok(Err(e)) => return Err(format!("Error reading from stdout: {}", e)),
                }
            },
            stderr_result = tokio::time::timeout(Duration::from_millis(50), stderr.read(&mut error_buf)) => {
                match stderr_result {
                    Ok(Ok(0)) | Err(_) => {},
                    Ok(Ok(bytes_read)) => {
                        error.extend_from_slice(&error_buf[..bytes_read]);
                        error_bytes_read = bytes_read;
                    },
                    Ok(Err(e)) => return Err(format!("Error reading from stderr: {}", e)),
                }
            },
        }
        if have_the_token && error_bytes_read == 0 { break; }
    }

    Ok((String::from_utf8_lossy(&output).to_string(), String::from_utf8_lossy(&error).to_string(), have_the_token))
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

pub async fn kill_process_and_children(process: &Child, process_name4log: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let process_id = process.id().ok_or("Failed to get process id")? as i32;
        let pid = Pid::from_raw(process_id);
        kill(pid, Signal::SIGTERM).map_err(|e| format!("Failed to kill '{process_name4log}' and its children. Error: {}", e))?;
    }

    // todo: test on windows
    #[cfg(windows)]
    {
        use std::process::Command;
        let pid = process.id().expect("Failed to get process id");
        Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .map_err(|e| format!("Failed to kill tool '{process_name4log}' and its children. Error: {}. Try Again", e))?;
    }

    Ok(())
}
