use super::source::{self, InstalledSource, TestResult};
use super::window_source;
use census_crawl::participation_evidence::recorded::read_recorded_independent_participation_window;
use census_crawl::participation_evidence::{ParticipationEvidenceRefV1, SubjectOwnershipV1};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootV1, RetainedCaptureSlotV1,
    Store,
};

fn observe(
    store: &Store,
    reference: &ParticipationEvidenceRefV1,
) -> TestResult<(RetainedCaptureRootV1, String, String)> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let recorded =
        read_recorded_independent_participation_window(&snapshot, reference, &mut grant)?
            .ok_or("controlled full-window source missing")?;
    let checked = recorded.checked();
    check!(eq; checked.subject().name(), "Controlled Target");
    check!(eq; checked.subject().school(), "Detroit Catholic Central");
    check!(eq; checked.from().as_str(), "2026-03-01");
    check!(eq; checked.through().as_str(), "2026-05-31");
    check!(eq; checked.documented_summer_end().as_str(), "2026-05-31");
    check!(eq; checked.school_year().get(), 2025);
    check!(eq; checked.cohort().get(), 2027);
    match checked.ownership() {
        SubjectOwnershipV1::ProviderLink(person) => {
            check!(eq; person.url.as_deref(), Some("https://mi.milesplit.com/athletes/9999999-controlled-target"))
        }
        SubjectOwnershipV1::DocumentOnly(_) => {
            return Err("controlled provider ownership missing".into())
        }
    }
    let leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: recorded.root().clone(),
        slot: RetainedCaptureSlotV1::ParticipationWindow,
        kind: RetainedCaptureLeafKindV1::RootJson,
        object: recorded.root().payload.clone(),
    };
    snapshot.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    Ok((
        recorded.root().clone(),
        checked.boundary_proof().capture().into(),
        checked.boundary_proof().interpretation().into(),
    ))
}

#[test]
fn controlled_full_window_preserves_explicit_dates_and_source_proof_after_reopen() -> TestResult {
    let installed = source::install()?;
    let reference = window_source::install(&installed)?;
    let before = observe(&installed.store, &reference)?;
    source::require_originals(&installed.store, &installed.source_records)?;
    let InstalledSource {
        directory,
        store,
        source_records,
        candidate,
        public,
        ..
    } = installed;
    drop(store);
    let reopened = source::reopen(&directory)?;
    source::require_originals(&reopened, &source_records)?;
    source::require_identity(&reopened, &candidate, &public)?;
    check!(eq; observe(&reopened, &reference)?, before);
    Ok(())
}
