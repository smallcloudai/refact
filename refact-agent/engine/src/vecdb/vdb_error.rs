use tokio::time::{sleep, Duration};
use std::future::Future;
use tracing::warn;

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