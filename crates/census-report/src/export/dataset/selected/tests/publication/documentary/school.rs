use super::fixture::Fixture;
use super::TestResult;
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, read_recorded_provider_school,
};
use census_crawl::participation_evidence::result_ownership::school_correspondence::{
    build_claim, ResultSchoolSourceProposalV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use super::super::controlled_three_marks::school_issuance::scope_for;

pub(super) fn propose(fixture: &Fixture) -> TestResult<ResultSchoolSourceProposalV1> {
    let reference = fixture.provider_school_reference()?;
    let snapshot = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let provider = read_recorded_provider_school(&snapshot, &reference, &mut grant)?
        .ok_or("recorded provider school absent in genuine store")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let discovered = discover_current_html_results(&current, &mut grant)?;
    check!(eq; discovered.group_count(), 1);
    check!(eq; discovered.diagnostic_count(), 0);
    let proposal = discovered.with_group(0, &mut grant, |linked, grant| {
        let row = linked.row(0).ok_or(census_store::StoreError::Refused {
            detail: "genuine school row missing".into(),
        })?;
        let scope = scope_for(linked.checked_document(), row.checked_row())?;
        build_claim(&current, &linked, 0, &provider, scope, grant)
    })?;
    Ok(proposal)
}