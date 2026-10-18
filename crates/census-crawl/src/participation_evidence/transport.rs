use super::{bound, invalid};
use census_store::StoreResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedHttpProvenanceV1 {
    pub request_method: String,
    pub request_headers: Vec<(String, String)>,
    pub response_header_bytes: Vec<u8>,
    pub content_type: String,
    pub body_encoding: RetainedBodyEncodingV1,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetainedBodyEncodingV1 {
    Identity,
}

impl RetainedHttpProvenanceV1 {
    pub(super) fn validate(&self, completed_at: DateTime<Utc>) -> StoreResult<()> {
        bound(
            self.request_headers.len(),
            32,
            "participation request header count",
        )?;
        bound(
            self.response_header_bytes.len(),
            16 * 1024,
            "participation response header bytes",
        )?;
        bound(self.content_type.len(), 128, "participation MIME bytes")?;
        self.validate_request_headers()?;
        if self.request_method != "GET"
            || self.started_at > completed_at
            || !self
                .content_type
                .eq_ignore_ascii_case("text/html; charset=utf-8")
        {
            return Err(invalid("participation transport provenance is unsupported"));
        }
        self.validate_response_headers()
    }

    fn validate_request_headers(&self) -> StoreResult<()> {
        self.request_headers.iter().try_for_each(|(key, value)| {
            bound(key.len(), 128, "participation request header name")?;
            bound(value.len(), 2048, "participation request header value")?;
            if key.is_empty()
                || key.bytes().any(|b| b.is_ascii_control())
                || value.bytes().any(|b| b.is_ascii_control())
            {
                return Err(invalid("participation request header is invalid"));
            }
            Ok(())
        })
    }

    fn validate_response_headers(&self) -> StoreResult<()> {
        let headers = std::str::from_utf8(&self.response_header_bytes)
            .map_err(|_| invalid("participation response headers are not UTF-8"))?;
        let blocks: Vec<_> = headers
            .split("\r\n\r\n")
            .filter(|value| !value.is_empty())
            .collect();
        let final_headers = blocks
            .last()
            .ok_or_else(|| invalid("participation response headers absent"))?;
        let mut lines = final_headers.split("\r\n");
        let status = lines
            .next()
            .ok_or_else(|| invalid("participation response status absent"))?;
        if !matches!(
            status.split_whitespace().next(),
            Some("HTTP/1.1" | "HTTP/2")
        ) || status.split_whitespace().nth(1) != Some("200")
        {
            return Err(invalid("participation header status differs"));
        }
        let mut mime = false;
        lines.try_for_each(|line| {
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| invalid("participation response header malformed"))?;
            if name.eq_ignore_ascii_case("content-type") {
                if !value.trim().eq_ignore_ascii_case(&self.content_type) {
                    return Err(invalid("participation MIME differs from retained headers"));
                }
                mime = true;
            }
            if name.eq_ignore_ascii_case("content-encoding")
                && !value.trim().eq_ignore_ascii_case("identity")
            {
                return Err(invalid(
                    "participation encoded HTTP representation unsupported",
                ));
            }
            Ok(())
        })?;
        if !mime {
            return Err(invalid("participation response MIME header absent"));
        }
        Ok(())
    }
}
