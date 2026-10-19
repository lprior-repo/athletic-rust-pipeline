pub(super) mod lifecycle;
use super::super::super::super::selection;
use super::super::super::super::source::{InstalledSource, TestResult};
use crate::report::Scope;
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, read_recorded_independent_participation,
};
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::{
    build_claim, ResultCalendarSourceProposalV1,
};
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::QueryProfileV1;
use census_domain::participation_candidates::ParticipationReason;
use census_store::StoreError;
pub(super) use lifecycle::{bridge_all, publish_selected, reopen, revoke_only_50};
pub(super) fn propose_for(
    installed: &InstalledSource,
    reference: &ParticipationEvidenceRefV1,
    row: usize,
) -> TestResult<ResultCalendarSourceProposalV1> {
    let locator = installed
        .row_locators
        .get(row)
        .ok_or("controlled row missing")?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let requested = read_recorded_independent_participation(&snapshot, reference, &mut grant)?
        .ok_or("calendar documentary capture missing")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let discovered = discover_current_html_results(&current, &mut grant)?;
    let mut index = 0;
    while index < discovered.group_count() {
        let found = discovered.with_group(index, &mut grant, |linked, grant| {
            if linked.recorded_root().key.as_str() != locator.document_key.as_str() {
                return Ok(None);
            }
            let row = linked.row(0).ok_or(StoreError::Refused {
                detail: "controlled calendar row missing".into(),
            })?;
            if row.checked_row().locator() != locator.locator {
                return Err(StoreError::Refused {
                    detail: "controlled calendar row locator differs".into(),
                }
                .into());
            }
            build_claim(&current, &linked, 0, &requested, grant).map(Some)
        })?;
        if let Some(proposal) = found {
            return Ok(proposal);
        }
        index += 1;
    }
    Err("controlled calendar result group missing".into())
}
pub(super) fn require_withholds(
    installed: &InstalledSource,
    profile: &QueryProfileV1,
) -> TestResult {
    let objects = tempfile::tempdir()?;
    let dataset = selection::capture(&installed.store, profile, objects.path())?;
    check!(eq; dataset.performances.len(), 3);
    let feb = dataset
        .performances
        .iter()
        .find(|row| crate::bests::mark_text(&row.mark) == "49.00" && row.date == "2026-02-28")
        .ok_or("controlled Feb28 absent")?;
    let may50 = dataset
        .performances
        .iter()
        .find(|row| crate::bests::mark_text(&row.mark) == "50.00" && row.date == "2026-05-15")
        .ok_or("controlled May15 absent")?;
    let may51 = dataset
        .performances
        .iter()
        .find(|row| crate::bests::mark_text(&row.mark) == "51.00" && row.date == "2026-05-16")
        .ok_or("controlled May16 absent")?;
    let view = dataset.verified_participation()?;
    let feb_assessment = view.assessment(&feb.id).ok_or("Feb28 assessment")?;
    check!(
        eq;
        feb_assessment.outcome(),
        crate::export::ParticipationOutcome::Excluded
    );
    check!(feb_assessment
        .reasons()
        .contains(ParticipationReason::ParticipationContextUnresolved));
    check!(feb_assessment
        .reasons()
        .contains(ParticipationReason::OutsideParticipationWindow));
    check!(eq; feb_assessment.reasons().iter().count(), 2);
    for owned in [may50, may51] {
        let assessment = view.assessment(&owned.id).ok_or("May assessment")?;
        check!(
            eq;
            assessment.outcome(),
            crate::export::ParticipationOutcome::Unresolved
        );
        check!(assessment
            .reasons()
            .iter()
            .eq([ParticipationReason::ParticipationContextUnresolved,]));
        check!(assessment.diagnostics().is_empty());
    }
    let authority = dataset.selected_authority()?;
    let selected = authority.profile();
    let [cohort] = selected.input().graduation_years.as_slice() else {
        return Err("cohort absent".into());
    };
    let outcome = crate::bests::build_with_exclusions(
        &dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year: Some(cohort.get()),
            limit: None,
            jurisdictions: selected.input().jurisdictions.clone(),
        },
    )?;
    check!(outcome.selections.is_empty());
    Ok(())
}
pub(super) fn require_feb_exact(dataset: &crate::export::ExportDataset) -> TestResult {
    let feb = dataset
        .performances
        .iter()
        .find(|row| crate::bests::mark_text(&row.mark) == "49.00" && row.date == "2026-02-28")
        .ok_or("Feb28 absent")?;
    let view = dataset.verified_participation()?;
    let assessment = view.assessment(&feb.id).ok_or("Feb28 assessment")?;
    check!(
        eq;
        assessment.outcome(),
        crate::export::ParticipationOutcome::Excluded
    );
    check!(assessment
        .reasons()
        .iter()
        .eq([ParticipationReason::OutsideParticipationWindow,]));
    check!(assessment.diagnostics().is_empty());
    Ok(())
}
