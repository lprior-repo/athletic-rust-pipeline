use super::reopen;
use super::source::{
    install, require_identity, require_originals, InstalledSource, RowLocator, TestResult,
};
use census_crawl::participation_evidence::recorded::read_recorded_html_result_capture;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalAthlete, EventKind, ExactSeconds, Gender, Mark};
use census_store::{Store, Table};

fn verify_mark(store: &Store, locator: &RowLocator, mark: &str, day: &str) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let capture = read_recorded_html_result_capture(
        &snapshot,
        &locator.document_key,
        &[locator.locator],
        &mut grant,
    )?
    .ok_or("retained controlled result absent")?;
    let document = capture.read(&[locator.locator])?;
    check!(eq; document.published_start(), day);
    let row = document.row(0).ok_or("controlled result row absent")?;
    check!(eq; row.parsed_row().mark, Mark::TimeSeconds(ExactSeconds::parse(mark)?));
    check!(eq; row.event().kind, EventKind::Track400m);
    check!(eq; row.event().gender, Gender::Boys);
    check!(eq; row.parsed_row().name, "Controlled Target");
    check!(eq; row.parsed_row().school, "Detroit Catholic Central");
    check!(eq; row.parsed_row().grade.map(|grade| grade.get()), Some(11));
    Ok(())
}

fn verify_rows(store: &Store, locators: &[RowLocator]) -> TestResult {
    check!(eq; locators.len(), 3);
    for (locator, (mark, day)) in locators.iter().zip([
        ("49.00", "2026-02-28"),
        ("50.00", "2026-05-15"),
        ("51.00", "2026-05-16"),
    ]) {
        verify_mark(store, locator, mark, day)?;
    }
    Ok(())
}

#[test]
fn controlled_three_marks_installs_exact_sources_and_source_bound_identity() -> TestResult {
    let installed = install()?;
    require_identity(&installed.store, &installed.candidate, &installed.public)?;
    let athlete: CanonicalAthlete = installed
        .store
        .snapshot()
        .get_merged(Table::Athletes, installed.candidate.as_str())?
        .ok_or("source-bound controlled athlete absent")?;
    check!(eq; athlete.grad_year.get(), 2027);
    check!(eq; athlete.canonical_name, "Controlled Target");
    check!(eq; athlete.school, installed.school);
    require_originals(&installed.store, &installed.source_records)?;
    verify_rows(&installed.store, &installed.row_locators)?;
    let InstalledSource {
        directory,
        store,
        candidate,
        public,
        row_locators,
        source_records,
        ..
    } = installed;
    drop(store);
    let reopened = reopen(&directory)?;
    require_identity(&reopened, &candidate, &public)?;
    verify_rows(&reopened, &row_locators)?;
    require_originals(&reopened, &source_records)?;
    drop(reopened);
    directory.close()?;
    Ok(())
}
