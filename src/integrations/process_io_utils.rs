use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStdin};
use tokio::time::{timeout, Duration, sleep};
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

pub async fn read_until_token_or_timeout<R>(buffer: &mut R, timeout_ms: u64, token: &str) -> Result<String, String>
where
    R: AsyncReadExt + Unpin,
{
    let mut output = Vec::new();
    let mut buf = [0u8; 1024];
    
    loop {
        let read_result = if timeout_ms > 0 {
            timeout(Duration::from_millis(timeout_ms), buffer.read(&mut buf)).await
        } else {
            Ok(buffer.read(&mut buf).await)
        };

        let bytes_read = match read_result {
            Ok(Ok(bytes)) => bytes,                      // Successfully read
            Ok(Err(e)) => return Err(e.to_string()),     // Read error
            Err(_) => return Ok(String::from_utf8_lossy(&output).to_string()), // Timeout, return current output
        };

        if bytes_read == 0 { break; }
        
        output.extend_from_slice(&buf[..bytes_read]);

        if !token.is_empty() && output.trim_ascii_end().ends_with(token.as_bytes()) { break; }
    }

    Ok(String::from_utf8_lossy(&output).to_string())
}

pub async fn wait_until_port_gets_busy(port: u16, timeout_duration: &Duration) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", port);

    let result: Result<_, _> = timeout(timeout_duration.clone(), async {
        loop {
            match TcpStream::connect(&addr).await {
                Ok(_) => {
                    return Ok::<(), std::io::Error>(());
                },
                Err(_) => sleep(Duration::from_millis(500)).await,
            }
        }
    }).await;

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(format!("Timeout expired: Port {} wasn't busy after {:?}", port, timeout_duration)),
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
        let pid = process.id();
        Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .map_err(|e| format!("Failed to kill tool '{process_name4log}' and its children. Error: {}. Try Again", e))?;
    }

    Ok(())
}
