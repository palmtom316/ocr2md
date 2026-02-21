use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode, header::HeaderMap};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::config::RuntimeConfig;
use crate::error::AppError;

#[derive(Clone)]
pub struct HttpEngine {
    client: Client,
    config: RuntimeConfig,
}

impl HttpEngine {
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.request_timeout_ms))
            .build()
            .context("failed to build reqwest client")?;
        Ok(Self { client, config })
    }

    pub async fn post_json(
        &self,
        service: &str,
        url: &str,
        headers: HeaderMap,
        payload: &Value,
        trace_id: &str,
    ) -> Result<Value> {
        let body = serde_json::to_vec(payload).context("failed to serialize request payload")?;

        let mut last_err: Option<anyhow::Error> = None;

        for attempt in 0..=self.config.retry_max {
            let started = Instant::now();

            let response = self
                .client
                .post(url)
                .headers(headers.clone())
                .body(body.clone())
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.context("failed reading response body")?;
                    let latency = started.elapsed().as_millis();

                    info!(
                        service,
                        url,
                        status = status.as_u16(),
                        latency_ms = latency,
                        trace_id,
                        "http_response"
                    );

                    if status.is_success() {
                        let parsed = serde_json::from_str::<Value>(&text)
                            .with_context(|| format!("invalid JSON from {service}"))?;
                        return Ok(parsed);
                    }

                    let retryable_status = is_retryable_status(status);
                    if retryable_status && attempt < self.config.retry_max {
                        let delay_ms = self.backoff_ms(attempt);
                        warn!(
                            service,
                            url,
                            status = status.as_u16(),
                            attempt,
                            delay_ms,
                            trace_id,
                            "transient_status_retry"
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }

                    return Err(AppError::ApiStatus {
                        status: status.as_u16(),
                        message: truncate_for_error(&text),
                    }
                    .into());
                }
                Err(err) => {
                    let retryable_error = is_retryable_reqwest_error(&err);

                    if retryable_error && attempt < self.config.retry_max {
                        let delay_ms = self.backoff_ms(attempt);
                        warn!(
                            service,
                            url,
                            attempt,
                            delay_ms,
                            trace_id,
                            error = %err,
                            "transport_retry"
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }

                    last_err = Some(err.into());
                    break;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("unknown HTTP error")))
    }

    fn backoff_ms(&self, attempt: u32) -> u64 {
        self.config
            .retry_base_ms
            .saturating_mul(2u64.saturating_pow(attempt))
    }
}

pub fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_reqwest_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

fn truncate_for_error(content: &str) -> String {
    const MAX: usize = 800;
    if content.chars().count() <= MAX {
        return content.to_string();
    }

    let mut buf = String::with_capacity(MAX + 32);
    for ch in content.chars().take(MAX) {
        buf.push(ch);
    }
    buf.push_str("...(truncated)");
    buf
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;

    use super::is_retryable_status;

    #[test]
    fn retryable_status_rule() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }
}
