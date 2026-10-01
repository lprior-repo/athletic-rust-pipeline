use std::fmt;
use std::future::Future;
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 10;
const CONNECT_TIMEOUT_SECS: u64 = 15;
const USER_AGENT: &str = "athletic-rust-pipeline-school-address/0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportFailure {
    pub detail: String,
}

impl fmt::Display for TransportFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.detail.as_str())
    }
}

impl std::error::Error for TransportFailure {}

pub trait Transport {
    fn get_json(
        &self,
        url: &str,
        authorization: Option<&str>,
    ) -> impl Future<Output = Result<String, TransportFailure>>;
}

pub struct HttpTransport {
    client: reqwest::Client,
}

impl HttpTransport {
    pub fn new() -> Result<Self, TransportFailure> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .build()
            .map_err(sanitize)?;
        Ok(Self { client })
    }
}

fn sanitize(error: reqwest::Error) -> TransportFailure {
    TransportFailure {
        detail: error.without_url().to_string(),
    }
}

async fn body_text(response: reqwest::Response) -> Result<String, TransportFailure> {
    let response = response.error_for_status().map_err(sanitize)?;
    response.text().await.map_err(sanitize)
}

impl Transport for HttpTransport {
    async fn get_json(
        &self,
        url: &str,
        authorization: Option<&str>,
    ) -> Result<String, TransportFailure> {
        let mut request = self.client.get(url);
        if let Some(authorization) = authorization {
            let value = reqwest::header::HeaderValue::from_str(authorization).map_err(|error| {
                TransportFailure {
                    detail: error.to_string(),
                }
            })?;
            request = request.header(reqwest::header::AUTHORIZATION, value);
        }
        let response = request.send().await.map_err(sanitize)?;
        body_text(response).await
    }
}
