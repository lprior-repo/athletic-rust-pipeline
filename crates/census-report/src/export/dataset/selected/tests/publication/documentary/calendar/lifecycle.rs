use super::super::super::controlled_three_marks::{
    calendar_lifecycle::revocation, school_issuance,
};
use super::super::fixture::Fixture;
use super::super::TestResult;
use crate::export::ExportDataset;
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1;
use census_domain::model::{ExactSeconds, GradYear, Mark, TimingMethod};
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarPredecessorV1, ResultCalendarReviewCommandV1,
};

pub(super) fn prepare(fixture: &Fixture) -> TestResult<ResultCalendarSourceProposalV1> {
    school_issuance::enroll(
        &fixture.store,
        "genuine-documentary-school-enrollment",
        "Explicit genuine documentary school enrollment after retained status",
    )
    .map_err(|error| error as Box<dyn std::error::Error>)?;
    let school_proposal = super::super::school::propose(fixture)?;
    school_issuance::accept_one(
        &fixture.store,
        &school_proposal,
        "genuine-documentary-school-accept",
        "Explicit protected genuine documentary school review exact performance day only",
    )
    .map_err(|error| error as Box<dyn std::error::Error>)?;
    revocation::enroll_calendar(
        &fixture.store,
        "genuine-documentary-calendar-enrollment",
        "Explicit genuine documentary calendar enrollment after school9 and reviewed status",
    )
    .map_err(|error| error as Box<dyn std::error::Error>)?;
    super::propose(fixture)
}

pub(super) fn accept(
    fixture: &Fixture,
) -> TestResult<(ResultCalendarReviewCommandV1, census_store::Receipt)> {
    let proposal = prepare(fixture)?;
    revocation::apply_calendar_action(
        &fixture.store,
        &proposal,
        "genuine-documentary-calendar-accept",
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        "Explicit genuine documentary calendar review exact performance day only",
    )
    .map_err(|error| error as Box<dyn std::error::Error>)
}

fn require_row0(fixture: &Fixture, dataset: &ExportDataset) -> TestResult {
    let row = fixture
        .expected
        .iter()
        .find(|expected| {
            expected.references.iter().any(|reference| {
                reference == "bc7c7f83bcfc715c9c3343e09cf45f82bee7651ac86838f3427e1097de010b3d"
            })
        })
        .ok_or("genuine expected original result row")?;
    let performance = dataset
        .performances
        .iter()
        .find(|performance| performance.id == row.performance)
        .ok_or("genuine performance projection")?;
    check!(eq; performance.date, "2026-05-15");
    check!(eq; performance.timing, Some(TimingMethod::Fat));
    check!(eq; performance.wind_mps, Some(0.2));
    check!(eq; performance.observed_grade.map(|grade| grade.get()), Some(12));
    check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("13.57")?));
    super::school_oracle::require_historical_school(dataset, performance)?;
    let view = dataset.verified_participation()?;
    let assessment = view
        .assessment(&row.performance)
        .ok_or("participation assessment for genuine original result")?;
    check!(eq; assessment.outcome(), crate::export::ParticipationOutcome::Eligible);
    check!(assessment.reasons().is_empty());
    Ok(())
}

#[test]
fn native_genuine_documentary_co2026_calendar_acceptance_projects_eligible_participation(
) -> TestResult {
    let fixture = Fixture::open_for(GradYear::new(2026).ok_or("genuine Class-of-2026 cohort")?)?;
    accept(&fixture)?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    require_row0(&fixture, &dataset)?;
    fixture.require_original_unchanged()?;
    fixture.close()?;
    objects.close()?;
    Ok(())
}
