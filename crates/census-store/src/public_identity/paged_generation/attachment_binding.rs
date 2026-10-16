use super::{capture, invalid, CaptureBindingV2, MAX_SEAL_JSONL_BYTES};
use crate::public_identity::codec;
use crate::{Store, StoreError, StoreResult, StoreSnapshot};

pub(crate) fn attachment_capture_binding(
    store: &Store,
    snapshot: &StoreSnapshot<'_>,
) -> StoreResult<CaptureBindingV2> {
    snapshot
        .revision()
        .ensure_origin(store, "attachment capture binding")?;
    capture::binding::capture(store, snapshot)
}

pub(crate) fn decode_attachment_capture_binding(raw: &[u8]) -> StoreResult<CaptureBindingV2> {
    let max = usize::try_from(MAX_SEAL_JSONL_BYTES).map_err(|_| StoreError::CounterOverflow)?;
    if raw.len() > max {
        return Err(StoreError::RowTooLarge {
            bytes: raw.len(),
            max,
        });
    }
    let binding = CaptureBindingV2::from_wire(codec::decode("attachment capture binding", raw)?)?;
    if codec::encode(&binding, max)? != raw {
        return Err(invalid("noncanonical attachment capture binding"));
    }
    Ok(binding)
}

pub(crate) fn require_attachment_capture_inputs(
    original: &CaptureBindingV2,
    current: &CaptureBindingV2,
    expected_evidence: u64,
    expected_derived: u64,
) -> StoreResult<()> {
    capture::binding::require_inputs(original, current)?;
    if original.evidence_generation() != expected_evidence
        || current.evidence_generation() != expected_evidence
        || original.derived_generation() != expected_derived
        || current.derived_generation() != expected_derived
    {
        return Err(invalid("attachment capture generations moved"));
    }
    Ok(())
}

pub(crate) fn require_attachment_retained_inputs(
    raw: &[u8],
    capture: &CaptureBindingV2,
    store: &census_domain::model::StoreIdentity,
    expected_evidence: u64,
    expected_derived: u64,
) -> StoreResult<()> {
    let token =
        crate::input_revisions::decode_attachment_token(raw, expected_evidence, expected_derived)?;
    capture.require_identity_graph_v3_format()?;
    if capture.store_identity() != store
        || capture.evidence_generation() != expected_evidence
        || capture.derived_generation() != expected_derived
    {
        return Err(invalid(
            "attachment historical input lineage or generations",
        ));
    }
    for (revision, table) in capture
        .0
        .table_revisions
        .iter()
        .zip(super::AuthorityTableV2::ALL)
    {
        if *revision != token.table_revision(table.table())? {
            return Err(invalid("attachment historical input table revision"));
        }
    }
    Ok(())
}
