use tokio::time::{sleep, Duration};
use std::future::Future;
use tracing::{warn, error};

pub fn map_sqlite_error(e: tokio_rusqlite::Error, operation: &str) -> String {
    match e {
        tokio_rusqlite::Error::Rusqlite(rusqlite::Error::SqliteFailure(error, Some(msg))) => {
            format!("SQLite error during {}: {} (extended code: {})", operation, msg, error.extended_code)
        },
        tokio_rusqlite::Error::Rusqlite(rusqlite::Error::SqliteFailure(error, None)) => {
            format!("SQLite error during {}: code {}", operation, error.extended_code)
        },
        tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => {
            format!("SQLite query during {} returned no rows", operation)
        },
        tokio_rusqlite::Error::Rusqlite(e) => {
            format!("SQLite error during {}: {}", operation, e)
        },
        tokio_rusqlite::Error::ConnectionClosed => {
            format!("SQLite connection for operation {} was closed", operation)
        },
        tokio_rusqlite::Error::Other(_) => {
            format!("SQLite operation {} encountered an error", operation)
        },
        _ => format!("Error during {}: {}", operation, e),
    }
}

pub async fn with_retry<F, Fut, T, E>(
    operation: F, 
    max_retries: usize, 
    retry_delay: Duration, 
    op_name: &str
) -> Result<T, String>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    loop {
        match operation().await {
            Ok(res) => return Ok(res),
            Err(err) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(format!("Operation {} failed after {} attempts: {}", op_name, max_retries, err));
                }
                warn!("Operation {} failed at attempt {}/{}: {}. Retrying in {:?}...", 
                    op_name, attempts, max_retries, err, retry_delay);
                sleep(retry_delay).await;
            }
        }
    }
}

pub fn log_and_fallback<T: Default>(err: impl std::fmt::Display, context: &str) -> T {
    error!("{}: {}", context, err);
    T::default()
}