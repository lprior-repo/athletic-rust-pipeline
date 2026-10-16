use super::{profile, ExportDataset, Store, TestResult};
use crate::export::public_identity::tests::fixture::{self, Fixture};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, ProfileSelectionV1};
use census_store::ReviewedIdentityCommitOutcomeV1;

mod identity;
mod roster;

#[derive(Clone, Copy)]
pub(crate) enum RosterCase {
    Current,
    Missing,
    Undated,
    OutsideCaptureDay,
    Coop,
    Conflicted,
}

pub(crate) fn source_births() -> TestResult<Fixture> {
    source_births_with(RosterCase::Current)
}

pub(crate) fn source_births_with(case: RosterCase) -> TestResult<Fixture> {
    let fixture = fixture::source_births()?;
    fixture.store.register_profile(&profile()?)?;
    if matches!(case, RosterCase::Missing) {
        return Ok(fixture);
    }
    let school = school(case);
    let [first, _, _] = &fixture.athletes;
    let first_roster = roster::append(&fixture.store, first, &school, case)?;
    identity::attach(&fixture, first, &first_roster)?;
    if matches!(case, RosterCase::Conflicted) {
        let mut other = CanonicalSchool::new(
            census_domain::UsJurisdiction::Wisconsin,
            "Synthetic Conflicting Wisconsin High",
            "synthetic conflicting wisconsin high",
            None,
        )
        .0;
        other
            .source_identities
            .push(census_domain::model::SourceIdentity::new(
                census_domain::model::SourceNamespace::MilesplitSchool,
                "52650",
            ));
        let competing = roster::append(&fixture.store, first, &other, RosterCase::Current)?;
        identity::attach(&fixture, first, &competing)?;
    }
    Ok(fixture)
}

pub(crate) fn capture(store: &Store) -> TestResult<(ExportDataset, tempfile::TempDir)> {
    let objects = tempfile::tempdir()?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile()?.id()?);
    let dataset = ExportDataset::from_selected_snapshot(
        store,
        &store.snapshot(),
        &selection,
        objects.path(),
    )?;
    dataset.ensure_current_selected_write()?;
    Ok((dataset, objects))
}

pub(crate) fn merge(fixture: &Fixture) -> TestResult<ReviewedIdentityCommitOutcomeV1> {
    let request = fixture::prepare_merge(fixture)?;
    let expected = fixture.writer.head(&fixture.store)?;
    let result = fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "report-reviewed-merge",
        &request,
        fixture.store.snapshot().revision(),
        &expected,
    )?;
    check!(eq; result.head().sequence, expected.sequence.checked_add(1).ok_or("head sequence")?);
    Ok(result)
}

fn school(case: RosterCase) -> CanonicalSchool {
    let mut school = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Synthetic Current Wisconsin High",
        "synthetic current wisconsin high",
        None,
    )
    .0;
    school.co_op = matches!(case, RosterCase::Coop);
    school
        .source_identities
        .push(census_domain::model::SourceIdentity::new(
            census_domain::model::SourceNamespace::MilesplitSchool,
            "52649",
        ));
    school
}
