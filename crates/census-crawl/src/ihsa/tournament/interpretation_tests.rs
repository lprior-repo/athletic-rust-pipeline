use super::journal::{Journal, PHASE};
use super::map::Mapper;
use super::wire::{EventRow, MeetRow, QualifiersEnvelope};
use crate::{net::Fetcher, AdapterContext};
use census_domain::model::{
    serialized_digest, CanonicalEvent, EventIdentity, EventKind, EventSpecification, SchoolYear,
};
use census_store::{Store, Table};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const STAMP: &str = "2026-09-20T14:39:00Z";
const URL: &str = "https://api.ihsa.org/v1/track-field/meets/2026/events?gender=Boys";

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    capture: &'a crate::ResultsCaptureContext,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        capture: crate::CaptureContext::DurableV1(capture),
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2025).ok_or("controlled school year")?,
        observed_on: "2026-09-30".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
            .ok_or("controlled cutoff")?,
        recording: None,
    })
}

fn meet_row() -> MeetRow {
    MeetRow {
        meet_id: 74003,
        year: "2026".into(),
        title: "Captured IHSA Meet".into(),
        gender: "Boys".into(),
        last_refreshed_at: Some("2026-05-31T17:51:14.655Z".into()),
    }
}

#[test]
fn historical_tournament_completions_are_preserved_when_parser_three_claims_new_credit(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let ctx = context(&store, &fetcher, &capture)?;
    let old_meet = json!({"key":"meet:74003","url":URL,"parser":2,"parsed":true,
        "refreshed_at":meet_row().last_refreshed_at,"performance_as_of":ctx.performance_as_of});
    let old_list = json!({"key":"qualifiers:2025-26:688","url":URL,"parser":2,"parsed":true});
    store.journal_done("ihsa_tournament", "meet:74003", &old_meet)?;
    store.journal_done("ihsa_tournament", "qualifiers:2025-26:688", &old_list)?;
    check!(eq; PHASE, "ihsa_tournament_v3");
    let mut journal = Journal::load(&ctx)?;
    check!(eq; journal.meets, HashMap::new());
    check!(eq; journal.qualifiers, HashSet::new());
    store.journal_done(
        PHASE,
        "qualifiers:2025-26:888",
        &json!({"key":"qualifiers:2025-26:888","parser":2,"parsed":true}),
    )?;
    store.journal_done(
        PHASE,
        "qualifiers:2025-26:999",
        &json!({"key":"qualifiers:2025-26:999","parser":3,"parsed":false}),
    )?;
    check!(eq; Journal::load(&ctx)?.qualifiers, HashSet::new());
    journal.meet(&meet_row(), URL, (1, ctx.performance_as_of));
    let list: QualifiersEnvelope = serde_json::from_value(json!({"tournamentId":"688"}))?;
    journal.list("2025-26:688", URL, &list);
    Mapper::load(&ctx, "ihsa")?.store(&ctx, journal.take_pending())?;
    let current = Journal::load(&ctx)?;
    check!(eq; current.meets, HashMap::from([("74003".into(), meet_row().last_refreshed_at)]));
    check!(eq; current.qualifiers, HashSet::from(["2025-26:688".into()]));
    for key in ["meet:74003", "qualifiers:2025-26:688"] {
        let payload = store
            .journal_payload(PHASE, key)?
            .ok_or("current completion")?;
        check!(eq; payload.get("parser"), Some(&json!(3)));
        check!(eq; payload.get("parsed"), Some(&json!(true)));
    }
    check!(eq; store.journal_payload("ihsa_tournament", "meet:74003")?, Some(old_meet));
    check!(eq; store.journal_payload("ihsa_tournament", "qualifiers:2025-26:688")?, Some(old_list));
    Ok(())
}

fn event_row() -> TestResult<EventRow> {
    Ok(serde_json::from_value(json!({
        "eventId":"1","eventType":"IndividualEvent","gender":"M","classDivision":"1A",
        "eventName":"Boys 3000m XC 1A - Finals","round":"F","hasResults":false,
        "scheduledDate":"2026-05-30T00:00:00.000Z"
    }))?)
}

fn mapped<'a>(ctx: &AdapterContext<'_>) -> TestResult<Mapper<'a>> {
    let mut mapper = Mapper::load(ctx, "ihsa")?;
    mapper.origin.observed_on = STAMP.into();
    let meet = mapper.meet(&meet_row(), "2026-05-30", None, URL);
    mapper.event(&meet, &event_row()?, URL)?;
    Ok(mapper)
}

fn legacy_witness<T: serde::Serialize>(
    store: &Store,
    table: Table,
    row: &T,
) -> TestResult<(String, Value)> {
    let digest = serialized_digest(row)?;
    let key = format!("{}/{digest}", table.file());
    let payload = json!({"table":table.file(),"content_digest":digest});
    store.append(table, row)?;
    store.journal_done("ihsa_tournament_fact_effects_v1", &key, &payload)?;
    Ok((key, payload))
}

#[test]
fn old_fact_witnesses_cannot_hide_the_new_xc_interpretation_or_duplicate_its_replay() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let ctx = context(&store, &fetcher, &capture)?;
    let mapper = mapped(&ctx)?;
    let meet = mapper
        .accumulated
        .meets
        .values()
        .next()
        .ok_or("captured meet")?;
    let event = mapper
        .accumulated
        .events
        .values()
        .next()
        .ok_or("current event")?;
    check!(eq; event.kind, EventKind::CrossCountry);
    let mut old_event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track3000m,
            gender: event.gender,
            division: event.division.as_deref(),
            round: event.round.as_deref(),
        },
        EventSpecification::default(),
    )?;
    old_event.source_labels = event.source_labels.clone();
    old_event.evidence = event.evidence.clone();
    let old_meet = legacy_witness(&store, Table::Meets, meet)?;
    let old_fact = legacy_witness(&store, Table::Events, &old_event)?;
    let old_digest = serialized_digest(&old_event)?;
    let current_id = event.id.clone();
    let counts = mapper.store(&ctx, Vec::new())?;
    check!(eq; (counts.meets,counts.events), (1,1));
    check!(eq; store.walk_table(Table::Meets)?.rows, 2);
    check!(eq; store.walk_table(Table::Events)?.rows, 2);
    let events: Vec<CanonicalEvent> = store.snapshot().scan(Table::Events)?;
    check!(eq; events.len(), 2);
    let preserved = events
        .iter()
        .find(|event| event.id == old_event.id)
        .ok_or("old interpretation")?;
    check!(eq; serialized_digest(preserved)?, old_digest);
    let current = events
        .iter()
        .find(|event| event.id == current_id)
        .ok_or("new interpretation")?;
    check!(eq; current.kind, EventKind::CrossCountry);
    check!(eq; current.evidence.first().ok_or("physical stamp")?.observed_on, STAMP);
    let effects = store.journal_keys("ihsa_tournament_fact_effects_v2")?;
    check!(eq; effects.len(), 2);
    check!(effects.contains(&old_meet.0));
    let replay = mapped(&ctx)?.store(&ctx, Vec::new())?;
    check!(eq; (replay.meets,replay.events), (0,0));
    check!(eq; store.walk_table(Table::Meets)?.rows, 2);
    check!(eq; store.walk_table(Table::Events)?.rows, 2);
    check!(eq; store.journal_keys("ihsa_tournament_fact_effects_v2")?, effects);
    for (key, payload) in [old_meet, old_fact] {
        check!(eq; store.journal_payload("ihsa_tournament_fact_effects_v1",&key)?, Some(payload));
    }
    Ok(())
}
