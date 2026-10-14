use super::{persist, Accumulator, PHASE};
use crate::athleticnet::absorb::{absorb, AbsorbContext, AbsorbOutcome};
use crate::athleticnet::map::Stats;
use crate::athleticnet::{Bio, Scope, Target};
use crate::{net::Fetcher, AdapterContext};
use census_domain::model::{CanonicalEvent, EventKind, SchoolYear, SourceRef};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::HashMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn cutoff() -> TestResult<chrono::NaiveDate> {
    chrono::NaiveDate::from_ymd_opt(2026, 9, 30).ok_or_else(|| "controlled cutoff".into())
}

fn profile_rows() -> TestResult<Accumulator> {
    let bio: Bio = serde_json::from_value(serde_json::json!({
        "athlete":{"IDAthlete":28872883,"FirstName":"Captured","LastName":"Runner",
            "Gender":"M","SchoolID":3204},
        "grades":{"3204_2026":11},
        "allTeams":{"3204":{"SchoolName":"Middleton"}},
        "allSeasons":[],"resultsTF":[],
        "meets":{"589334":{"MeetName":"Captured XC","EndDate":"2025-09-20"}},
        "resultsXC":[{"IDResult":1,"Result":"10:00.00","SchoolID":3204,
            "MeetID":589334,"Distance":3000}]
    }))?;
    let target = Target {
        athlete_id: 28872883,
        state: Some(UsJurisdiction::Wisconsin),
    };
    let source = SourceRef::new("athleticnet", Some(
        "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=xc&level=4".into()
    ));
    let mut rows = Accumulator::default();
    let outcome = absorb(
        &bio,
        Scope::CrossCountry,
        &target,
        AbsorbContext {
            source: &source,
            observed_on: "2026-09-20T12:00:00Z",
            performance_as_of: cutoff()?,
            index: &SchoolIndex::from_schools(&[]),
            resolved: &mut HashMap::new(),
            stats: &mut Stats::default(),
            accumulated: &mut rows,
        },
    )?;
    check!(eq; outcome, AbsorbOutcome::Complete { rows:1 });
    Ok(rows)
}

#[test]
fn prior_profile_effect_retains_its_receipt_while_current_xc_projection_remains_idempotent(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let ctx = AdapterContext {
        capture: crate::CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("controlled school year")?,
        observed_on: "2026-10-01".into(),
        performance_as_of: cutoff()?,
        recording: None,
    };
    let rows = profile_rows()?;
    let school = rows.schools.values().next().ok_or("captured school")?;
    check!(eq; ctx.append_row_once("net_profile_projection_v4", Table::Schools, school)?, true);
    let old_keys = store.journal_keys("crawl_effect_receipts_v1")?;
    check!(eq; old_keys.len(), 1);
    let old_key = old_keys.into_iter().next().ok_or("old effect key")?;
    check!(old_key.starts_with("net_profile_projection_v4:"));
    let old_receipt = store.receipt(&old_key)?.ok_or("old receipt")?;
    let old_payload = store.journal_payload("crawl_effect_receipts_v1", &old_key)?;
    let written = persist(&ctx, rows)?;
    check!(eq; PHASE, "net_profile_projection_v5");
    check!(eq; written.schools, 1);
    check!(eq; written.events, 1);
    check!(eq; written.performances, 1);
    check!(eq; store.walk_table(Table::Schools)?.rows, 2);
    let events: Vec<CanonicalEvent> = store.snapshot().scan(Table::Events)?;
    check!(eq; events.len(), 1);
    let event = events.first().ok_or("current event")?;
    check!(eq; event.kind, EventKind::CrossCountry);
    check!(eq; event.source_labels.first().ok_or("distance label")?.label, "Cross Country 3000m");
    check!(eq; event.evidence.first().ok_or("capture instant")?.observed_on, "2026-09-20T12:00:00Z");
    let effects = store.journal_keys("crawl_effect_receipts_v1")?;
    check!(effects
        .iter()
        .any(|key| key.starts_with("net_profile_projection_v5:")));
    let replay = persist(&ctx, profile_rows()?)?;
    check!(eq; (replay.schools, replay.events, replay.performances), (0,0,0));
    check!(eq; store.walk_table(Table::Schools)?.rows, 2);
    check!(eq; store.walk_table(Table::Events)?.rows, 1);
    check!(eq; store.walk_table(Table::Performances)?.rows, 1);
    check!(eq; store.journal_keys("crawl_effect_receipts_v1")?, effects);
    check!(eq; store.receipt(&old_key)?, Some(old_receipt));
    check!(eq; store.journal_payload("crawl_effect_receipts_v1", &old_key)?, old_payload);
    Ok(())
}
