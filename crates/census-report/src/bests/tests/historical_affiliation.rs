use super::*;
use census_domain::{JurisdictionBucket, MeetState};

mod artifacts;
mod fixture;
mod selection;
mod workbook;

use fixture::{AffiliationCase, Fixture};

const HISTORICAL_SCHOOL: &str = "Historical Wisconsin High";
const CURRENT_SCHOOL: &str = "Current Illinois High";
const ATHLETE_NAME: &str = "Historical affiliation runner";
const PROFILE_URL: &str = "https://example.test/milesplit/athletes/historical-affiliation";
const RESULT_URL: &str = "https://example.test/results/historical-affiliation";
const RESULT_DATE: &str = "2025-05-08";
const SOURCE_OWNER: &str = "historical-affiliation-athlete";
const SOURCE_KEY: &str = "wiaa_results:historical-affiliation:400m:final";
const MARK_VALUE: i64 = 48_550_000_000;

fn options(scope: Scope) -> Options {
    Options {
        scope,
        grad_year: Some(2027),
        limit: None,
    }
}

fn assert_selection(
    dataset: &ExportDataset,
    fixture: &Fixture,
    scope: Scope,
) -> TestResult<SharedSelection> {
    let rows = build_from_dataset(dataset, &options(scope));
    check!(eq; rows.len(), 1, "{:?} under {scope:?}", fixture.case);
    let row = rows.first().ok_or("missing historical-affiliation PR")?;
    check!(eq; row.athlete.school.as_deref(), fixture.case.school());
    check!(eq; row.athlete.athlete_school.as_str(), fixture.case.school_text());
    check!(eq; row.athlete.athlete_state, fixture.case.state());
    check!(eq; row.athlete.name.as_str(), ATHLETE_NAME);
    check!(eq; row.athlete.gender, Gender::Boys);
    check!(eq; row.athlete.grad_year, 2027);
    check!(eq; row.athlete.profile_url.as_deref(), Some(PROFILE_URL));
    assert_result(row, fixture)?;
    fixture.assert_dataset(dataset)?;
    Ok(row.clone())
}

fn assert_result(row: &SharedSelection, fixture: &Fixture) -> TestResult {
    let expected = &fixture.performance;
    check!(eq; row.athlete_id(), &fixture.athlete.id);
    check!(eq; row.key.event_kind, EventKind::Track400m);
    check!(eq; row.key.surface, crate::bests::SurfaceClass::Outdoor);
    check!(eq; row.key.timing, crate::bests::TimingClass::Fat);
    check!(eq; row.result.mark, expected.mark);
    check!(eq; row.result.value, MARK_VALUE);
    check!(eq; row.result.normalized, Some(48.55));
    check!(eq; row.mark_text(), "48.55");
    check!(eq; row.result.place, Some(1));
    check!(eq; row.result.timing, Some(TimingMethod::Fat));
    check!(eq; row.result.wind_mps, None);
    check!(eq; row.source.performance_id, expected.id);
    check!(eq; row.source.source_athlete.as_str(), SOURCE_OWNER);
    check!(eq; row.source.source_key.as_str(), SOURCE_KEY);
    check!(eq; row.source.result_url.as_str(), RESULT_URL);
    check!(eq; row.meet.meet_id, expected.meet);
    check!(eq; row.meet.name.as_str(), fixture.meet.name.as_str());
    check!(eq; row.meet.date.as_str(), RESULT_DATE);
    check!(eq; row.meet.meet_state, MeetState::Placed(UsJurisdiction::Michigan));
    check!(eq; row.meet.heat.as_deref(), Some("2"));
    check!(eq; row.meet.round.as_deref(), Some("Final"));
    check!(eq; row.population.marks, 1);
    check!(eq; row.population.sources, 1);
    check!(eq; row.conflicts, Vec::new());
    Ok(())
}
