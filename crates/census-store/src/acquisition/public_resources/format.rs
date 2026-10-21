use super::declaration::PublicResourceDeclarationV1;
use super::frontier::PublicResourceFrontierEntryV1;
use super::{refused, DECLARATION, ENTRY_PREFIX, FRONTIER_OPERATION_PREFIX, OPERATION_PREFIX};
use crate::{Receipt, StoreResult};
use census_domain::model::{CaptureQualification, QueryProfileV1, RunManifest};

mod startup;
pub(crate) use startup::{has_state, validate_format};

pub(crate) fn validate_declaration(
    record: &PublicResourceDeclarationV1,
    capture: &CaptureQualification,
    manifest: &RunManifest,
    profile: &QueryProfileV1,
    receipt: &Receipt,
) -> StoreResult<()> {
    record.binding.validate(capture, manifest, profile)?;
    record.validate(receipt)
}

pub(crate) fn validate_declaration_key(key: &[u8]) -> StoreResult<()> {
    if key != DECLARATION.as_bytes() {
        return Err(refused("public resource declaration namespace"));
    }
    Ok(())
}

pub(crate) fn validate_entry_key(
    key: &[u8],
    entry: &PublicResourceFrontierEntryV1,
) -> StoreResult<()> {
    let expected = format!("{ENTRY_PREFIX}{}", entry.id()?);
    if key != expected.as_bytes() {
        return Err(refused("public resource frontier namespace"));
    }
    Ok(())
}

pub(crate) fn validate_entry_receipt(
    declaration: &PublicResourceDeclarationV1,
    entry: &PublicResourceFrontierEntryV1,
    receipt: &Receipt,
) -> StoreResult<()> {
    entry.validate(declaration)?;
    let operation = format!("{FRONTIER_OPERATION_PREFIX}{}", entry.id()?);
    let at = chrono::DateTime::parse_from_rfc3339(&receipt.at)
        .map_err(|_| refused("public resource frontier receipt time"))?;
    if receipt.operation != operation
        || receipt.digest != super::codec::object_digest(entry)?.as_str()
        || at < declaration.admitted_at
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(refused("public resource frontier receipt"));
    }
    Ok(())
}

pub(crate) fn refuse_generic_phase(phase: &str) -> StoreResult<()> {
    if reserved_family(phase.as_bytes()) {
        return Err(refused("public resource owned phase required"));
    }
    Ok(())
}

pub(crate) fn refuse_generic_operation(operation: &str) -> StoreResult<()> {
    if is_reserved_operation(operation.as_bytes()) {
        return Err(refused("public resource owned operation required"));
    }
    Ok(())
}

pub(crate) fn is_reserved_operation(operation: &[u8]) -> bool {
    reserved_family(operation)
        || operation.starts_with(OPERATION_PREFIX.as_bytes())
        || operation.starts_with(FRONTIER_OPERATION_PREFIX.as_bytes())
}

pub(crate) fn reserved_family(key: &[u8]) -> bool {
    key.starts_with(b"public_resource_capture")
        || key.starts_with(b"public_resource_frontier")
        || key.starts_with(b"public_resource_child")
        || key.starts_with(b"public_resource_school")
}
