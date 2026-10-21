use super::{body, limit, refused, MHSAA_COOPS_SOURCE_URL};
use crate::participation_evidence::RetainedHttpProvenanceV1;
use census_store::StoreResult;
use chrono::{DateTime, Datelike, Utc};

pub(super) fn validate(
    requested: &str,
    final_url: &str,
    status: u16,
    acquired: DateTime<Utc>,
    bytes: usize,
    raw: &RetainedHttpProvenanceV1,
) -> StoreResult<()> {
    body::length(bytes)?;
    if requested != MHSAA_COOPS_SOURCE_URL || final_url != requested || status != 200 {
        return Err(refused(
            "MHSAA PDF requires the exact nonredirected source and HTTP200",
        ));
    }
    limit(
        raw.request_headers.len(),
        32,
        "MHSAA PDF request header count",
    )?;
    limit(
        raw.response_header_bytes.len(),
        16 * 1024,
        "MHSAA PDF response header bytes",
    )?;
    limit(raw.content_type.len(), 128, "MHSAA PDF content type bytes")?;
    if raw.request_method != "GET"
        || raw.started_at > acquired
        || !(1900..=2100).contains(&acquired.year())
        || !(1900..=2100).contains(&raw.started_at.year())
        || !raw.content_type.eq_ignore_ascii_case("application/pdf")
    {
        return Err(refused("MHSAA PDF Raw transport metadata differs"));
    }
    match raw.body_encoding {
        crate::participation_evidence::RetainedBodyEncodingV1::Identity => {}
    }
    raw.request_headers
        .iter()
        .try_for_each(|(name, value)| header(name, value))?;
    response(raw, bytes)
}

fn header(name: &str, value: &str) -> StoreResult<()> {
    limit(name.len(), 128, "MHSAA PDF header name bytes")?;
    limit(value.len(), 2048, "MHSAA PDF header value bytes")?;
    if name.is_empty() || !name.bytes().all(token) || value.bytes().any(|b| b.is_ascii_control()) {
        return Err(refused("MHSAA PDF header name or value is invalid"));
    }
    Ok(())
}

fn response(raw: &RetainedHttpProvenanceV1, bytes: usize) -> StoreResult<()> {
    let dump = std::str::from_utf8(&raw.response_header_bytes)
        .map_err(|_| refused("MHSAA PDF Raw header dump is not UTF8"))?;
    let block = dump
        .strip_suffix("\r\n\r\n")
        .ok_or_else(|| refused("MHSAA PDF Raw header dump is incomplete"))?;
    if block.is_empty() || block.contains("\r\n\r\n") {
        return Err(refused(
            "MHSAA PDF Raw dump contains redirect/intermediate heads",
        ));
    }
    let mut lines = block.split("\r\n");
    let status = lines
        .next()
        .ok_or_else(|| refused("MHSAA PDF Raw status is absent"))?;
    status_line(status)?;
    let mut seen = Seen::default();
    for (index, line) in lines.enumerate() {
        limit(index, 31, "MHSAA PDF response header count")?;
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| refused("MHSAA PDF response header is malformed"))?;
        header(name, value)?;
        seen.accept(name, value.trim(), bytes)?;
    }
    if !seen.mime {
        return Err(refused("MHSAA PDF MIME header is absent"));
    }
    Ok(())
}

fn status_line(line: &str) -> StoreResult<()> {
    if line.bytes().any(|b| b.is_ascii_control() || !b.is_ascii()) {
        return Err(refused("MHSAA PDF status line is invalid"));
    }
    let mut parts = line.split_whitespace();
    if !matches!(parts.next(), Some("HTTP/1.1" | "HTTP/2")) || parts.next() != Some("200") {
        return Err(refused("MHSAA PDF Raw response status differs"));
    }
    Ok(())
}

#[derive(Default)]
struct Seen {
    mime: bool,
    encoding: bool,
    length: bool,
}

impl Seen {
    fn accept(&mut self, name: &str, value: &str, bytes: usize) -> StoreResult<()> {
        if name.eq_ignore_ascii_case("content-type") {
            if self.mime || !value.eq_ignore_ascii_case("application/pdf") {
                return Err(refused("MHSAA PDF Raw MIME differs or is duplicated"));
            }
            self.mime = true;
        } else if name.eq_ignore_ascii_case("content-encoding") {
            if self.encoding || !value.eq_ignore_ascii_case("identity") {
                return Err(refused(
                    "MHSAA PDF encoded representation differs or is duplicated",
                ));
            }
            self.encoding = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            if self.length || value != bytes.to_string() {
                return Err(refused("MHSAA PDF body length differs from Raw headers"));
            }
            self.length = true;
        } else if name.eq_ignore_ascii_case("location") {
            return Err(refused("MHSAA PDF redirect location is unsupported"));
        }
        Ok(())
    }
}

fn token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}
