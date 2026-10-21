use super::*;
use crate::evidence_subject_attachment::external::CheckedCapture;
use crate::{RetainedCaptureLeafKindV1, RetainedCaptureRootV1, RetainedCaptureSlotV1};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

pub(super) fn require(
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    namespace: &str,
    root: &RetainedCaptureRootV1,
    capture: &CheckedCapture<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let statement = &source.statement;
    let Material::Retained { leaf } = object(materials, &statement.body)? else {
        return Err(Error::SourceMismatch("body must be retained original").into());
    };
    if leaf.version != 1
        || &leaf.root != root
        || leaf.slot != RetainedCaptureSlotV1::SelfCapture
        || leaf.kind != RetainedCaptureLeafKindV1::OriginalBody
    {
        return Err(Error::SourceMismatch("original body root/slot/kind").into());
    }
    let body = capture.body_bytes(&leaf.object, grant)?;
    let row = span(&body, statement.row_offset, statement.row_length)?;
    let quote = span(
        &body,
        statement.statement_offset,
        statement.statement_length,
    )?;
    let end = statement
        .row_offset
        .checked_add(statement.row_length)
        .ok_or(Error::BudgetExceeded)?;
    let locator = format!("html_bytes:{}:{end}", statement.row_offset);
    if codec::digest(row, grant)? != statement.row_digest
        || codec::digest(quote, grant)? != statement.statement_digest
        || quote != statement.statement_text.as_str().as_bytes()
        || codec::digest(locator.as_bytes(), grant)? != statement.row_locator
    {
        return Err(Error::SourceMismatch("original row/statement bytes").into());
    }
    interpretation(
        source,
        materials,
        namespace,
        capture.requested_url(grant)?.as_ref(),
        grant,
    )
}

fn span(body: &[u8], offset: u64, length: u64) -> StoreResult<&[u8]> {
    let end = offset.checked_add(length).ok_or(Error::BudgetExceeded)?;
    let start = usize::try_from(offset).map_err(|_| Error::BudgetExceeded)?;
    let end = usize::try_from(end).map_err(|_| Error::BudgetExceeded)?;
    body.get(start..end)
        .ok_or_else(|| Error::SourceMismatch("source byte range").into())
}

fn interpretation(
    source: &ResultSchoolSourceClaimV1,
    materials: &[Material],
    namespace: &str,
    requested_url: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let bytes = object(materials, &source.statement.interpretation)?.inline_bytes(grant)?;
    let parser_bytes = object(materials, &source.statement.parser_contract)?.inline_bytes(grant)?;
    let parser = ResultSchoolSourceParserV1::from_contract(&parser_bytes)
        .ok_or(Error::SourceMismatch("school parser contract"))?;
    let (contract, url, body, locator): (&str, &str, &str, &RawValue) =
        serde_json::from_slice(&bytes)
            .map_err(|_| Error::Corrupt("row interpretation preimage"))?;
    grant.charge_bytes(bytes.len()).map_err(grant_error)?;
    if contract != parser.row_interpretation()
        || namespace != parser.document_namespace()
        || body != source.statement.body.digest.as_str()
    {
        return Err(Error::SourceMismatch("row interpretation binding").into());
    }
    let fields: BTreeMap<&str, &RawValue> =
        serde_json::from_str(locator.get()).map_err(|_| Error::Corrupt("source row locator"))?;
    let ordinal: u32 = field(&fields, "ordinal")?;
    let offset: u64 = field(&fields, "byte_offset")?;
    let length: u64 = field(&fields, "byte_length")?;
    let expected =
        format!("{{\"ordinal\":{ordinal},\"byte_offset\":{offset},\"byte_length\":{length}}}");
    if fields.len() != 3
        || locator.get() != expected
        || offset != source.statement.row_offset
        || length != source.statement.row_length
    {
        return Err(Error::SourceMismatch("row locator preimage").into());
    }
    let canonical = codec::encode(&(contract, url, body, locator), grant)?;
    if canonical != bytes {
        return Err(Error::Corrupt("noncanonical interpretation preimage").into());
    }
    if requested_url != url {
        return Err(Error::SourceMismatch("interpretation requested URL").into());
    }
    Ok(())
}

fn field<'a, T: serde::Deserialize<'a>>(
    fields: &BTreeMap<&str, &'a RawValue>,
    name: &str,
) -> StoreResult<T> {
    let value = fields
        .get(name)
        .ok_or(Error::Corrupt("missing source field"))?;
    serde_json::from_str(value.get())
        .map_err(|_| Error::Corrupt("source field representation").into())
}
