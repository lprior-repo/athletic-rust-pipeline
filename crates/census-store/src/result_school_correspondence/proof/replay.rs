use super::super::*;
use super::{archive, capture, encoding, retained, ReplayedResultSchoolCorrespondencesV1};
use census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1;
use census_domain::model::StoreIdentity;

pub fn replay_result_school_source_proof_v1<'a>(
    bytes: &'a [u8],
    expected: &AttachmentObjectReferenceV1,
    store: &StoreIdentity,
    selected_snapshot_sequence: u64,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ReplayedResultSchoolCorrespondencesV1<'a>> {
    guarded(grant, |grant| {
        replay(bytes, expected, store, selected_snapshot_sequence, grant)
    })
}

fn replay<'a>(
    bytes: &'a [u8],
    expected: &AttachmentObjectReferenceV1,
    store: &StoreIdentity,
    sequence: u64,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ReplayedResultSchoolCorrespondencesV1<'a>> {
    if bytes.is_empty()
        || bytes.len()
            > census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes
        || expected.bytes != u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)?
    {
        return Err(Error::Corrupt("source closure length").into());
    }
    grant.charge_bytes(bytes.len()).map_err(grant_error)?;
    if codec::digest(bytes, grant)? != expected.digest {
        return Err(Error::Corrupt("source closure digest").into());
    }
    let archive: archive::Archive<'_> =
        serde_json::from_slice(bytes).map_err(|_| Error::Corrupt("source closure decoding"))?;
    if archive.schema == "result-school-source-closure-v1" {
        return Err(Error::Corrupt("unqualified source closure v1 unsupported").into());
    }
    if archive.schema != archive::SCHEMA
        || &archive.store != store
        || archive.snapshot_sequence != sequence
    {
        return Err(Error::Corrupt("source closure origin/snapshot/schema").into());
    }
    encoding::require_canonical(&archive, bytes, grant)?;
    let ledger = records::load(&archive, grant)?;
    if &ledger.store != store {
        return Err(Error::Corrupt("source closure enrollment origin").into());
    }
    grant
        .charge_bytes(archive.identity.get().len())
        .map_err(grant_error)?;
    let accepted = crate::replay_frozen_public_identity_v2(
        archive.identity.get().as_bytes(),
        &archive.identity_digest,
    )?;
    if accepted.store_identity() != store || accepted.snapshot_sequence() != sequence {
        return Err(Error::Corrupt("frozen identity origin/snapshot").into());
    }
    let attachments =
        crate::FrozenCurrentEvidenceSubjectAttachmentReferencesV1::from_borrowed_bytes(
            archive.attachments.get().as_bytes(),
            &archive.attachment_reference,
            accepted,
            grant,
        )?;
    require_roots(&archive, &ledger, &attachments, grant)?;
    let source = retained::Source::new(archive.external, grant)?;
    {
        let source = retained::Combined::new(&source, attachments.source());
        let records = attachments.retained_records(grant)?;
        ledger
            .records
            .iter()
            .zip(&ledger.contexts)
            .try_for_each(|(record, materials)| {
                super::super::source::require_accepted_head(
                    &record.command.accepted_head,
                    attachments.accepted_snapshot(),
                    grant,
                )?;
                super::super::source::require_historical(
                    &source,
                    &record.command.source,
                    materials,
                    records,
                    grant,
                )
            })?;
    }
    let continuation = grant.checkpoint().map_err(grant_error)?;
    Ok(ReplayedResultSchoolCorrespondencesV1 {
        ledger,
        source,
        attachments,
        continuation,
        snapshot_sequence: sequence,
    })
}

mod records;

fn require_roots(
    archive: &archive::Archive<'_>,
    ledger: &super::super::issuance::ledger::Ledger,
    attachments: &crate::FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let roots = capture::roots(ledger, grant)?;
    let mut external = archive.external.iter();
    roots.iter().try_for_each(|expected| {
        if attachments.contains_retained_capture_root(expected, grant)? {
            return Ok(());
        }
        let actual = external
            .next()
            .ok_or(Error::Corrupt("source closure root inventory"))?;
        if expected != &actual.root {
            return Err(Error::Corrupt("source closure root order/binding").into());
        }
        Ok::<_, StoreError>(())
    })?;
    if external.next().is_some() {
        return Err(Error::Corrupt("source closure root inventory").into());
    }
    Ok(())
}
