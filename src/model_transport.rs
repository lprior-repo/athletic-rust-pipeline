use crate::{extract::bounded_response_body, retry_policy};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::time::SystemTime;

const MAX_ATTEMPTS: u32 = 3;

pub async fn post_json(client: &Client, endpoint: &str, body: &Value) -> Result<Vec<u8>> {
    for attempt in 1..=MAX_ATTEMPTS {
        match client.post(endpoint).json(body).send().await {
            Ok(response) if response.status().is_success() => {
                match bounded_response_body(response).await {
                    Ok(bytes) => return Ok(bytes),
                    Err(error)
                        if attempt < MAX_ATTEMPTS
                            && error
                                .downcast_ref::<reqwest::Error>()
                                .is_some_and(transient_transport) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(response) => {
                let status = response.status();
                if !retry_policy::transient_status(status) || attempt == MAX_ATTEMPTS {
                    anyhow::bail!("local model returned HTTP {status} after {attempt} attempt(s)");
                }
                let delay = retry_policy::retry_after(response.headers(), SystemTime::now())?;
                drop(response);
                tokio::time::sleep(delay.max(retry_policy::backoff(attempt))).await;
                continue;
            }
            Err(error) if attempt < MAX_ATTEMPTS && transient_transport(&error) => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("calling local model at {endpoint} after {attempt} attempt(s)")
                })
            }
        }
        tokio::time::sleep(retry_policy::backoff(attempt)).await;
    }
    anyhow::bail!("local model exhausted retry budget")
}

fn transient_transport(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_body()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn transient_model_failure_exhausts_exactly_three_attempts() -> Result<()> {
        let mut server = mockito::Server::new_async().await;
        let failure = server
            .mock("POST", "/")
            .with_status(503)
            .expect(3)
            .create_async()
            .await;
        let result = post_json(&Client::new(), &server.url(), &serde_json::json!({})).await;
        assert!(result.is_err());
        failure.assert_async().await;
        Ok(())
    }
    #[tokio::test]
    async fn permanent_model_failure_is_not_retried() -> Result<()> {
        let mut server = mockito::Server::new_async().await;
        let failure = server
            .mock("POST", "/")
            .with_status(400)
            .expect(1)
            .create_async()
            .await;
        assert!(
            post_json(&Client::new(), &server.url(), &serde_json::json!({}))
                .await
                .is_err()
        );
        failure.assert_async().await;
        Ok(())
    }
}
