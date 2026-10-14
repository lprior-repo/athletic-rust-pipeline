use super::{OwnedCompleteness, OwnedMeetPage, OwnedRejection, OwnedRejectionKind, MAX_OWNED_ROWS};
use serde_json::Value;
use std::collections::HashSet;
use std::io::{self, Write};

#[path = "admission.rs"]
pub(super) mod admission;
pub(super) mod error;
pub use error::{
    OwnedCaptureResourceFailure, OwnedCaptureRetention, OwnedDocumentFailure, OwnedMeetParseError,
    OwnedParseArithmetic, OwnedParseLocator, OwnedParseResource, OwnedParseResourceFailure,
    OwnedResourceReceipt,
};

pub fn parse_owned_meet(body: &[u8], meet_id: u64) -> Result<OwnedMeetPage, OwnedMeetParseError> {
    if meet_id == 0 {
        return Err(OwnedDocumentFailure::InvalidRequestedMeet {
            locator: OwnedParseLocator::RequestedMeet,
        }
        .into());
    }
    admission::check(body)?;
    let mut document: Value =
        serde_json::from_slice(body).map_err(|source| OwnedDocumentFailure::Json {
            locator: OwnedParseLocator::Document,
            source,
        })?;
    check_envelope(&document, meet_id)?;
    let data = document
        .as_object_mut()
        .and_then(|object| object.remove("data"))
        .and_then(|data| match data {
            Value::Array(rows) => Some(rows),
            _ => None,
        })
        .ok_or(OwnedDocumentFailure::DataArray {
            locator: OwnedParseLocator::Data,
        })?;
    OwnedParseResourceFailure::limit(
        OwnedParseResource::ResultRows,
        OwnedParseLocator::Data,
        data.len(),
        MAX_OWNED_ROWS,
    )?;
    parse_rows(data, meet_id).map_err(Into::into)
}

fn parse_rows(data: Vec<Value>, meet_id: u64) -> Result<OwnedMeetPage, OwnedParseResourceFailure> {
    let mut page = reserved_page(data.len())?;
    let mut seen = HashSet::new();
    seen.try_reserve(data.len()).map_err(|source| {
        OwnedParseResourceFailure::allocation(
            OwnedParseResource::ResultIdentities,
            OwnedParseLocator::Data,
            data.len(),
            source,
        )
    })?;
    data.into_iter().enumerate().for_each(|(index, value)| {
        let locator = format!("data[{index}]");
        match super::row::parse_row(value, meet_id, locator.clone()) {
            Ok(row) if seen.insert(row.result_id) => page.rows.push(row),
            Ok(row) => page.rejected.push(OwnedRejection {
                locator,
                kind: OwnedRejectionKind::DuplicateResult,
                detail: format!("repeated source result ID {}", row.result_id),
            }),
            Err((kind, detail)) => page.rejected.push(OwnedRejection {
                locator,
                kind,
                detail,
            }),
        }
    });
    Ok(page)
}

fn reserved_page(published_rows: usize) -> Result<OwnedMeetPage, OwnedParseResourceFailure> {
    let mut page = OwnedMeetPage {
        rows: Vec::new(),
        rejected: Vec::new(),
        published_rows,
        completeness: OwnedCompleteness::Unknown,
    };
    page.rows
        .try_reserve_exact(published_rows)
        .map_err(|source| {
            OwnedParseResourceFailure::allocation(
                OwnedParseResource::AcceptedRows,
                OwnedParseLocator::Data,
                published_rows,
                source,
            )
        })?;
    page.rejected
        .try_reserve_exact(published_rows)
        .map_err(|source| {
            OwnedParseResourceFailure::allocation(
                OwnedParseResource::RejectedRows,
                OwnedParseLocator::Data,
                published_rows,
                source,
            )
        })?;
    Ok(page)
}

fn check_envelope(document: &Value, meet_id: u64) -> Result<(), OwnedMeetParseError> {
    if let Some(status) = document.pointer("/_meta/status_code") {
        if status.as_u64() != Some(200) {
            return Err(OwnedDocumentFailure::ProviderStatus {
                locator: OwnedParseLocator::ProviderStatus,
                published: diagnostic(status, OwnedParseLocator::ProviderStatus)?,
            }
            .into());
        }
    }
    let Some(published) = document.pointer("/_embedded/meet/id") else {
        return Ok(());
    };
    let id = published
        .as_u64()
        .or_else(|| published.as_str().and_then(|id| id.parse::<u64>().ok()));
    if id != Some(meet_id) {
        return Err(OwnedDocumentFailure::EmbeddedMeet {
            locator: OwnedParseLocator::EmbeddedMeet,
            published: diagnostic(published, OwnedParseLocator::EmbeddedMeet)?,
            requested: meet_id,
        }
        .into());
    }
    Ok(())
}

const MAX_DIAGNOSTIC_BYTES: usize = 1024;

enum DiagnosticSize {
    Exact(usize),
    Overflow,
}

struct Diagnostic {
    text: String,
    size: DiagnosticSize,
}

impl Write for Diagnostic {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.size = match self.size {
            DiagnosticSize::Exact(prior) => match prior.checked_add(bytes.len()) {
                Some(size) => DiagnosticSize::Exact(size),
                None => DiagnosticSize::Overflow,
            },
            DiagnosticSize::Overflow => DiagnosticSize::Overflow,
        };
        if matches!(self.size, DiagnosticSize::Exact(size) if size <= MAX_DIAGNOSTIC_BYTES) {
            let text = std::str::from_utf8(bytes).map_err(io::Error::other)?;
            self.text.push_str(text);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn diagnostic(value: &Value, locator: OwnedParseLocator) -> Result<String, OwnedMeetParseError> {
    let mut output = Diagnostic {
        text: String::new(),
        size: DiagnosticSize::Exact(0),
    };
    output
        .text
        .try_reserve_exact(MAX_DIAGNOSTIC_BYTES)
        .map_err(|source| {
            OwnedParseResourceFailure::allocation(
                OwnedParseResource::EnvelopeDiagnostic,
                locator,
                MAX_DIAGNOSTIC_BYTES,
                source,
            )
        })?;
    serde_json::to_writer(&mut output, value)
        .map_err(|source| OwnedDocumentFailure::Json { locator, source })?;
    let requested = match output.size {
        DiagnosticSize::Exact(size) => size,
        DiagnosticSize::Overflow => {
            return Err(OwnedParseResourceFailure::arithmetic(
                OwnedParseResource::EnvelopeDiagnostic,
                locator,
                OwnedParseArithmetic::DiagnosticBytes,
            )
            .into())
        }
    };
    OwnedParseResourceFailure::limit(
        OwnedParseResource::EnvelopeDiagnostic,
        locator,
        requested,
        MAX_DIAGNOSTIC_BYTES,
    )?;
    Ok(output.text)
}
