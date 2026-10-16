use super::{bound, hash, invalid, RetainedHttpProvenanceV1, MAX_BODY};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_store::StoreResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedHtmlCaptureV1 {
    requested_url: String,
    final_url: String,
    status: u16,
    acquired_at: DateTime<Utc>,
    body_bytes: usize,
    body_sha256: String,
    body_base64: String,
    provenance: RetainedHttpProvenanceV1,
}

impl RetainedHtmlCaptureV1 {
    pub fn new(
        requested_url: &str,
        final_url: &str,
        status: u16,
        acquired_at: DateTime<Utc>,
        body: &[u8],
        provenance: RetainedHttpProvenanceV1,
    ) -> StoreResult<Self> {
        provenance.validate(acquired_at)?;
        bound(body.len(), MAX_BODY, "participation HTML body bytes")?;
        url(requested_url)?;
        url(final_url)?;
        if requested_url != final_url || status != 200 || body.is_empty() {
            return Err(invalid(
                "participation HTML requires an exact nonredirected HTTP 200 body",
            ));
        }
        Ok(Self {
            requested_url: requested_url.into(),
            final_url: final_url.into(),
            status,
            acquired_at,
            body_bytes: body.len(),
            body_sha256: hash(body),
            body_base64: STANDARD.encode(body),
            provenance,
        })
    }

    pub fn body_sha256(&self) -> &str {
        &self.body_sha256
    }
    pub fn requested_url(&self) -> &str {
        &self.requested_url
    }
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }
    pub fn provenance(&self) -> &RetainedHttpProvenanceV1 {
        &self.provenance
    }

    pub(super) fn body(&self) -> StoreResult<String> {
        self.provenance.validate(self.acquired_at)?;
        bound(self.body_bytes, MAX_BODY, "participation HTML body bytes")?;
        bound(
            self.body_base64.len(),
            MAX_BODY * 2,
            "participation encoded body bytes",
        )?;
        url(&self.requested_url)?;
        url(&self.final_url)?;
        if self.status != 200 || self.requested_url != self.final_url || self.body_bytes == 0 {
            return Err(invalid("participation HTML capture ownership differs"));
        }
        let body = STANDARD
            .decode(&self.body_base64)
            .map_err(|_| invalid("participation HTML base64 is invalid"))?;
        if body.len() != self.body_bytes || hash(&body) != self.body_sha256 {
            return Err(invalid("participation HTML body length or digest differs"));
        }
        String::from_utf8(body).map_err(|_| invalid("participation HTML is not UTF-8"))
    }
}

fn url(value: &str) -> StoreResult<()> {
    bound(value.len(), 2048, "participation source URL bytes")?;
    if value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(invalid("participation URL contains control bytes"));
    }
    let parsed = url::Url::parse(value).map_err(|_| invalid("participation URL is invalid"))?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || parsed.port().is_some()
    {
        return Err(invalid("participation URL ownership is unsupported"));
    }
    Ok(())
}
