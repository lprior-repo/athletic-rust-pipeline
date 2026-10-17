use super::super::{ledger::Ledger, RetainedCaptureRootV1, OPERATION_PREFIX, PHASE};
use super::*;
use crate::read::RetainedJournalReadSource;
use crate::{AcceptedPublicIdentitySnapshot, StoreSnapshot};
use census_domain::evidence_subject_attachment::AttachmentInventoryV1;
use sha2::{Digest, Sha256};
use std::io::Write;

mod layout;
mod physical;
mod sink;
use physical::{Group, Summary, Totals};
use sink::{Count, Output};

pub(in super::super) fn capture(
    view: &StoreSnapshot<'_>,
    ledger: &Ledger<'_>,
    accepted: &AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<FrozenEvidenceSubjectAttachmentBundleV1> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = checked(view, ledger, accepted, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn checked(
    view: &StoreSnapshot<'_>,
    ledger: &Ledger<'_>,
    accepted: &AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<FrozenEvidenceSubjectAttachmentBundleV1> {
    ledger.validate_projection(accepted.store_identity(), grant)?;
    let roots = roots::collect(ledger, grant)?;
    let summaries = measure(view, &roots, grant)?;
    let length = measured_layout(view, accepted, &ledger.inventory, &summaries, grant)?;
    bound("frozen attachment archive bytes", length, MAX_ARCHIVE_BYTES)?;
    steps(grant, 2)?;
    let mut output = Output::new(length)?;
    let result = encode(
        view,
        accepted,
        &ledger.inventory,
        &summaries,
        &mut output,
        grant,
    );
    output.require_healthy()?;
    result?;
    steps(grant, 1)?;
    let bytes = output.finish()?;
    grant.charge_bytes(64).map_err(Error::Domain)?;
    let digest = AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(&bytes)))
        .map_err(Error::Domain)?;
    let reference = AttachmentObjectReferenceV1 {
        digest,
        bytes: u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?,
    };
    Ok(FrozenEvidenceSubjectAttachmentBundleV1 { bytes, reference })
}

fn measured_layout(
    view: &StoreSnapshot<'_>,
    accepted: &AcceptedPublicIdentitySnapshot,
    inventory: &AttachmentInventoryV1,
    summaries: &[Summary<'_>],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<usize> {
    let mut count = Count::new();
    let result: StoreResult<()> = (|| {
        steps(grant, 1)?;
        layout::head(&mut count, view, accepted, inventory)?;
        for (index, summary) in summaries.iter().enumerate() {
            steps(grant, 1)?;
            layout::prefix(&mut count, index, summary)?;
            count.skip(summary.json_rows)?;
            sink::write(&mut count, b"]}")?;
        }
        sink::write(&mut count, b"]}")?;
        Ok(())
    })();
    count.require_healthy()?;
    result?;
    count.finish()
}

fn measure<'a>(
    view: &StoreSnapshot<'_>,
    roots: &[&'a RetainedCaptureRootV1],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<Summary<'a>>> {
    let mut summaries = Vec::new();
    let mut totals = Totals::default();
    for group in [Group::Attachment, Group::Receipts]
        .into_iter()
        .chain(roots.iter().copied().map(Group::External))
    {
        steps(grant, 1)?;
        bound(
            "frozen archive prefixes",
            summaries
                .len()
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?,
            MAX_PREFIXES,
        )?;
        summaries
            .len()
            .checked_add(1)
            .and_then(|count| count.checked_mul(std::mem::size_of::<Summary<'_>>()))
            .ok_or(StoreError::CounterOverflow)?;
        material::allocate(&mut summaries)?;
        let mut count = Count::new();
        let result = physical::scan(view, group, &mut totals, &mut count, grant);
        count.require_healthy()?;
        let mut summary = result?;
        summary.json_rows = count.finish()?;
        summaries.push(summary);
    }
    Ok(summaries)
}

fn encode(
    view: &StoreSnapshot<'_>,
    accepted: &AcceptedPublicIdentitySnapshot,
    inventory: &AttachmentInventoryV1,
    summaries: &[Summary<'_>],
    output: &mut Output,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    layout::head(output, view, accepted, inventory)?;
    let mut totals = Totals::default();
    for (index, expected) in summaries.iter().enumerate() {
        steps(grant, 1)?;
        layout::prefix(output, index, expected)?;
        let actual = physical::scan(view, expected.group, &mut totals, output, grant)?;
        steps(grant, 1)?;
        if actual.rows != expected.rows
            || actual.bytes != expected.bytes
            || actual.digest != expected.digest
        {
            return Err(Error::Corrupt("frozen capture physical view changed").into());
        }
        sink::write(output, b"]}")?;
    }
    sink::write(output, b"]}")
}
