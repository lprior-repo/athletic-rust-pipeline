use super::*;
use crate::export::qualification::current_affiliation::PinnedCurrentAffiliations;
use crate::export::qualification::participation::{
    ParticipationReplay, ParticipationReplayV4, PinnedParticipation,
};
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, with_current_attached_participation,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

#[test]
fn native_genuine_v4_acquisition_after_frozen_instant_refuses_without_archive_mutation(
) -> TestResult {
    let fixture = Fixture::open()?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    inspect(&dataset, &fixture.expected)?;
    let earliest = earliest_acquired(&fixture.store)?;
    let cutoff = earliest
        .checked_sub_signed(chrono::Duration::nanoseconds(1))
        .ok_or("cutoff")?;
    let proof = serde_json::to_value(dataset.selected_proofs.as_ref().ok_or("selected proofs")?)?;
    let descriptor = proof
        .get("participation_digest")
        .and_then(Value::as_str)
        .ok_or("descriptor")?;
    let authentic = ParticipationReplay::load(objects.path(), descriptor)?;
    let authentic_bytes = serde_json::to_vec(&authentic)?;
    check!(matches!(&authentic, ParticipationReplay::V4(_)));
    let mut wire = serde_json::to_value(&authentic)?;
    *wire
        .pointer_mut("/retained/retained/captured_at")
        .ok_or("frozen instant")? = serde_json::to_value(cutoff)?;
    let altered = ParticipationReplay::V4(serde_json::from_value::<ParticipationReplayV4>(wire)?);
    let altered_descriptor = altered.install(objects.path())?;
    let before = inventory(objects.path())?;
    let affiliations = PinnedCurrentAffiliations::read(&dataset, &fixture.store.snapshot())?;
    let result = PinnedParticipation::replay(
        &dataset,
        &fixture.profile,
        objects.path(),
        &altered_descriptor,
        &authentic.fingerprints(),
        affiliations.roster_context(),
    );
    check!(
        matches!(result, Err(crate::report::ReportError::Documentary { source })
        if matches!(*source, crate::report::DocumentarySourceError::Assessment(
            census_domain::participation_candidates::ParticipationInputError::AcquisitionAfterFrozenInstant)))
    );
    check!(eq; inventory(objects.path())?, before);
    check!(eq; serde_json::to_vec(&ParticipationReplay::load(objects.path(), descriptor)?)?, authentic_bytes);
    fixture.require_original_unchanged()?;
    fixture.close()
}

fn earliest_acquired(store: &census_store::Store) -> TestResult<DateTime<Utc>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    Ok(with_current_attached_participation(
        &current,
        &mut grant,
        |sources, grant| {
            let plan = discover_current_html_results(&current, grant)?;
            plan.with_group(0, grant, |linked, grant| {
                sources.with_row_facts(&linked, 0, grant, |facts, _| {
                    let item = facts.sources().first().ok_or_else(|| {
                        census_store::StoreError::Refused {
                            detail: "genuine attached status source absent".into(),
                        }
                    })?;
                    Ok(linked
                        .checked_document()
                        .capture()
                        .acquired_at()
                        .min(item.source().status_proof().acquired_at())
                        .min(item.source().season_proof().acquired_at()))
                })
            })
        },
    )?)
}

fn inventory(directory: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let path = entry?.path();
            let bytes = std::fs::read(&path)?;
            Ok((path, bytes))
        })
        .collect()
}
