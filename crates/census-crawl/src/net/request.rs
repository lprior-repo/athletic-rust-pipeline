//! Request construction: verbs, bodies, and the conditional-GET headers.

use super::cache::CacheMeta;
use super::{FetchError, FetchOptions, FetchOutcome, Fetcher, REQUEST_TIMEOUT_SECS};

/// Request payload for POSTs: a pre-serialized body, plus the content type it must be sent with.
#[derive(Debug, Clone)]
pub(super) enum RequestBody {
    Json(String),
    Form(String),
}

impl Fetcher {
    /// GET a URL with caching, robots enforcement and per-host politeness.
    #[tracing::instrument(skip(self, options), fields(url, method = "GET"))]
    pub async fn get(&self, url: &str, options: &FetchOptions) -> Result<FetchOutcome, FetchError> {
        tracing::Span::current().record("url", url);
        self.fetch("GET", url, None, options, REQUEST_TIMEOUT_SECS)
            .await
    }

    /// POST a JSON body; cached by body content so repeated runs are free.
    ///
    /// Elasticsearch-backed result platforms take the query in the body, so the cache key must
    /// include that body: two different queries against one endpoint are two different documents.
    #[tracing::instrument(skip(self, options, body), fields(url, method = "POST"))]
    pub async fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
        options: &FetchOptions,
    ) -> Result<FetchOutcome, FetchError> {
        tracing::Span::current().record("url", url);
        let encoded = serde_json::to_string(body).map_err(|source| FetchError::Encode {
            target: url.to_string(),
            source,
        })?;
        self.fetch(
            "POST",
            url,
            Some((encoded.clone(), RequestBody::Json(encoded))),
            options,
            REQUEST_TIMEOUT_SECS,
        )
        .await
    }

    /// POST a form body (`application/x-www-form-urlencoded`); cached by body content so repeated
    /// runs are free.
    ///
    /// Some association directories answer only to a form POST of a query field (the NSAA export
    /// screen is one), so the same content-addressed caching the JSON POST uses applies here: two
    /// different queries against one endpoint are two different documents.
    #[tracing::instrument(skip(self, options, form), fields(url, method = "POST"))]
    pub async fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
        options: &FetchOptions,
    ) -> Result<FetchOutcome, FetchError> {
        tracing::Span::current().record("url", url);
        let encoded = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(
                form.iter()
                    .map(|(name, value)| (name.as_str(), value.as_str())),
            )
            .finish();
        self.fetch(
            "POST",
            url,
            Some((encoded.clone(), RequestBody::Form(encoded))),
            options,
            REQUEST_TIMEOUT_SECS,
        )
        .await
    }
}

/// Build an HTTP request with headers, body, and conditional GET support.
pub(super) fn build_request<'a>(
    client: &'a reqwest::Client,
    method: &str,
    url: &str,
    body: Option<&'a RequestBody>,
    headers: &[(String, String)],
    cached: Option<&'a CacheMeta>,
    refresh: bool,
) -> Result<reqwest::RequestBuilder, FetchError> {
    let mut request = match method {
        "POST" => client.post(url),
        _ => client.get(url),
    };
    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }
    if method == "POST" {
        if let Some(payload) = body {
            match payload {
                RequestBody::Json(encoded) => {
                    request = request
                        .header("content-type", "application/json")
                        .body(encoded.clone());
                }
                RequestBody::Form(encoded) => {
                    request = request
                        .header("content-type", "application/x-www-form-urlencoded")
                        .body(encoded.clone());
                }
            }
        }
    }
    if let Some(meta) = cached {
        if !refresh {
            if let Some(etag) = &meta.etag {
                request = request.header("If-None-Match", etag.as_str());
            }
            if let Some(last_modified) = &meta.last_modified {
                request = request.header("If-Modified-Since", last_modified.as_str());
            }
        }
    }
    Ok(request)
}
