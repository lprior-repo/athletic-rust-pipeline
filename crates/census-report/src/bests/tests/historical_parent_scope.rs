use super::*;
use crate::report::Derivation;
use crate::workbook::PerformanceProjection;
use census_domain::{JurisdictionBucket, MeetState};

mod artifacts;
mod fixture;
mod selection;
mod workbook;

use fixture::{Case, Fixture};

const RESULT_URL: &str = "https://example.test/results/historical-parent-scope";
const PROFILE_URL: &str = "https://example.test/milesplit/athletes/historical-parent-scope";
const RESULT_DATE: &str = "2025-05-08";
const MARK_VALUE: i64 = 48_550_000_000;

fn options(scope: Scope) -> Options {
    Options {
        scope,
        grad_year: Some(2027),
        limit: None,
    }
}

fn assert_result(row: &SharedSelection, fixture: &Fixture) -> TestResult {
    check!(eq; row.athlete_id(), &fixture.athlete.id);
    check!(eq; row.athlete.name.as_str(), fixture.case.athlete_name());
    check!(eq; row.athlete.gender, Gender::Boys);
    check!(eq; row.athlete.grad_year, 2027);
    check!(eq; row.athlete.profile_url.as_deref(), Some(PROFILE_URL));
    check!(eq; row.athlete.school.as_deref(), fixture.case.school());
    check!(eq; row.athlete.athlete_school.as_str(), fixture.case.school_text());
    check!(eq; row.athlete.athlete_state, fixture.case.school_bucket());
    check!(eq; row.key.event_kind, EventKind::Track400m);
    check!(eq; row.key.surface, crate::bests::SurfaceClass::Outdoor);
    check!(eq; row.key.wind_class, crate::bests::WindClass::NotApplicable);
    check!(eq; row.key.timing, crate::bests::TimingClass::Fat);
    check!(eq; row.key.measure, crate::bests::Measure::Time);
    check!(eq; row.key.context, None);
    check!(eq; row.key.specification, expected_specification());
    check!(eq; row.key.comparison, crate::bests::ComparisonPolicy::Standard);
    check!(eq; row.result.mark, Mark::TimeSeconds(ExactSeconds::parse("48.55")?));
    check!(eq; row.result.value, MARK_VALUE);
    check!(eq; row.result.normalized, Some(48.55));
    check!(eq; row.mark_text(), "48.55");
    check!(eq; row.result.place, Some(1));
    check!(eq; row.result.timing, Some(TimingMethod::Fat));
    check!(eq; row.result.wind_mps, None);
    check!(eq; row.source.performance_id, fixture.performance.id);
    check!(eq; row.source.source_athlete.as_str(), fixture.case.owner());
    check!(eq; row.source.source_key, fixture.performance.source_key);
    check!(eq; row.source.result_url.as_str(), RESULT_URL);
    check!(eq; row.source.specification, expected_specification());
    check!(eq; row.meet.meet_id, fixture.meet.id);
    check!(eq; row.meet.name.as_str(), fixture.case.meet_name());
    check!(eq; row.meet.date.as_str(), RESULT_DATE);
    check!(eq; row.meet.meet_state, MeetState::Placed(fixture.case.venue_state()));
    check!(eq; row.meet.heat.as_deref(), Some("2"));
    check!(eq; row.meet.round.as_deref(), Some("Final"));
    check!(eq; row.population.marks, 1);
    check!(eq; row.population.sources, 1);
    check!(eq; row.conflicts, Vec::new());
    Ok(())
}

fn assert_selection(dataset: &ExportDataset, fixture: &Fixture, scope: Scope) -> TestResult {
    let rows = build_from_dataset(dataset, &options(scope));
    check!(eq; rows.len(), 1, "{:?}, {scope:?}", fixture.case);
    assert_result(
        rows.first().ok_or("missing retained historical PR")?,
        fixture,
    )
}

fn assert_projection(dataset: &ExportDataset, fixture: &Fixture, scope: Scope) -> TestResult {
    let derivation = Derivation::of(dataset, scope, Some(2027));
    check!(eq; derivation.athletes(), std::slice::from_ref(&fixture.athlete));
    check!(eq; derivation.performances(), &[&fixture.performance]);
    let row =
        serde_json::to_value(PerformanceProjection::of(&derivation).row(&fixture.performance))?;
    let (meet, state) = fixture.case.meet_metadata(scope);
    artifacts::json_text(
        &row,
        &[
            ("id", fixture.performance.id.as_str()),
            ("athlete_id", fixture.athlete.id.as_str()),
            ("athlete", fixture.case.athlete_name()),
            ("school", fixture.case.school_text()),
            ("meet_id", fixture.meet.id.as_str()),
            ("meet", meet),
            ("date", RESULT_DATE),
            (
                "sport",
                if fixture.case.event_label(scope).is_empty() {
                    ""
                } else {
                    "Track"
                },
            ),
            ("event", fixture.case.event_label(scope)),
            ("mark", "48.55"),
            ("timing", "Fat"),
            ("round", "Final"),
            ("source", "wiaa_results"),
            ("source_result", fixture.performance.source_key.as_str()),
            ("source_url", RESULT_URL),
        ],
    )?;
    check!(eq; row.get("state"), Some(&serde_json::to_value(state)?));
    check!(eq; row.get("grad_year").and_then(serde_json::Value::as_i64), Some(2027));
    check!(eq; row.get("normalized").and_then(serde_json::Value::as_f64), Some(48.55));
    check!(eq; row.get("place").and_then(serde_json::Value::as_u64), Some(1));
    Ok(())
}

fn expected_specification() -> EventSpecification {
    EventSpecification {
        cross_country: None,
        hurdles: None,
        implement: None,
        indoor_track: None,
        category: Some(census_domain::model::CompetitionCategory::Boys),
    }
}
