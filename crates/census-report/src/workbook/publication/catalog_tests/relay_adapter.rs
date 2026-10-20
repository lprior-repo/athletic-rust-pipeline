use super::{fixture, options, publish, TestResult};
use crate::bests;
use crate::export::ExportDataset;
use census_crawl::milesplit::{
    collect_result_sets, owned_meet_request, ResultSetOptions, ResultSetRef, ResultSetRequest, Site,
};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{AdapterContext, CaptureContext};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalTeam, EventIdentity, EventKind,
    EventSpecification, Evidence, ExactSeconds, Gender, GradYear, Mark, RelayMember, RelayResult,
    SchoolYear, SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;

const MEET: &str = "726001";
const RESULT_SET: &str = "1267001";
const RESULT_ID: u64 = 201783901;
const SPLIT_LABEL: &str = "400m Split";
const FETCHED_AT: &str = "2026-09-20T12:00:00Z";

#[tokio::test]
async fn provider_split_is_excluded_and_member_only_relay_survives_workbook_publication(
) -> TestResult {
    let fixture = fixture()?;
    let split_report = ingest_provider_split(&fixture.store).await?;
    check!(eq; split_report.errors, 1);
    check!(eq; split_report.rows, 0);
    check!(
        matches!(
            census_crawl::hytek::hytek_event_kind(SPLIT_LABEL),
            EventKind::Unmapped { .. }
        ),
        "the provider event adapter classifies a split label as unmapped"
    );

    let snapshot = fixture.store.snapshot();
    let school = snapshot
        .scan::<census_domain::model::CanonicalSchool>(Table::Schools)?
        .into_iter()
        .next()
        .ok_or("fixture school")?;
    let team = snapshot
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .next()
        .ok_or("fixture team")?;
    let owner = team.source_identities.first().ok_or("fixture team owner")?;
    let meet = snapshot
        .scan::<CanonicalMeet>(Table::Meets)?
        .into_iter()
        .next()
        .ok_or("fixture meet")?;
    let mut relay_only = CanonicalAthlete::new(
        &school.id,
        "Relay Only Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "relay_only_4004"),
    )?;
    relay_only
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::new(
                "synthetic_provider_fixture",
                Some("https://example.test/relay-only-runner".into()),
            ),
        });
    fixture.store.append(Table::Athletes, &relay_only)?;

    let source = SourceRef::new(
        "wiaa_results",
        Some("https://example.test/provider-relay-fixture".into()),
    );
    let mut relay_event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Relay4x400,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    relay_event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: "4x400m Relay".into(),
    });
    relay_event
        .evidence
        .push(Evidence::parsed(source.clone(), &meet.date));
    let relay = RelayResult::new(
        &team.id,
        &relay_event.id,
        &meet.id,
        Mark::TimeSeconds(ExactSeconds::parse_clock("3:28.00")?),
        "synthetic:member-only-4x400",
    )
    .with_source_team(owner.clone())
    .with_member(RelayMember::new(1, "Catalog runner").with_athlete(fixture.athlete.clone()))
    .with_member(RelayMember::new(2, "Relay Only Runner").with_athlete(relay_only.id.clone()))
    .with_member(RelayMember::new(3, "Third relay member"))
    .with_member(RelayMember::new(4, "Fourth relay member"))
    .with_evidence(Evidence::parsed(source, &meet.date));
    fixture.store.append(Table::Events, &relay_event)?;
    fixture.store.append(Table::RelayResults, &relay)?;
    drop(snapshot);

    let dataset = ExportDataset::load(&fixture.store)?;
    check!(
        !dataset
            .performances
            .iter()
            .any(|row| row.source_key == format!("milesplit_result:{RESULT_ID}")),
        "the provider split row never becomes an individual performance"
    );
    check!(
        !dataset
            .performances
            .iter()
            .any(|row| row.athlete == relay_only.id),
        "relay membership alone does not mint an athlete performance"
    );
    let selected = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;
    check!(eq; selected.len(), 1);
    check!(eq; selected[0].athlete_id(), &fixture.athlete);
    check!(
        selected
            .iter()
            .all(|best| best.key.event_kind != EventKind::Track400m),
        "no individual 400m PR is selected from the relay split"
    );

    let path = publish(&fixture, &dataset)?;
    let generation = path.parent().ok_or("published generation")?;
    crate::workbook::verify::verify_frozen(&path, &dataset, &options()?)?;
    let relay_lines = std::fs::read_to_string(generation.join("relays.jsonl"))?;
    let published: Vec<Value> = relay_lines
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let member_only = published
        .iter()
        .find(|row| row.pointer("/result/id").and_then(Value::as_str) == Some(relay.id.as_str()))
        .ok_or("member-only relay workbook row")?;
    check!(
        member_only
            .pointer("/result/members/1/athlete")
            .and_then(Value::as_str)
            == Some(relay_only.id.as_str()),
        "the relay workbook keeps its linked member: {member_only:?}"
    );
    let reopened = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    check!(
        !reopened
            .performances
            .iter()
            .any(|row| row.athlete == relay_only.id),
        "independent frozen reopen preserves relay-only status"
    );
    Ok(())
}

async fn ingest_provider_split(
    store: &census_store::Store,
) -> TestResult<census_crawl::AdapterReport> {
    let site = Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let url = format!(
        "https://{}/meets/{MEET}/results/{RESULT_SET}/raw",
        site.host()
    );
    let reference = ResultSetRef::parse(&url).ok_or("provider result-set reference")?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let owned_request = owned_meet_request(&fetcher, &reference, &FetchOptions::default())?;
    let row = json!({
        "id": RESULT_ID,
        "athleteId": "4004",
        "teamId": "400",
        "teamName": "Catalog School",
        "meetId": MEET,
        "meetResultsId": RESULT_SET,
        "gender": "M",
        "place": "1",
        "heat": "1",
        "firstName": "Relay",
        "lastName": "Only Runner",
        "gradYear": "2027",
        "eventName": SPLIT_LABEL,
        "eventCode": SPLIT_LABEL,
        "eventDistance": 400,
        "roundName": "Finals",
        "round": "f",
        "mark": "51.20",
        "divisionName": "High School",
        "profileUrl": "https://wi.milesplit.com/athletes/4004/relay-only-runner"
    });
    let owned = json!({
        "_meta": {"status_code": 200},
        "_embedded": {"meet": {"id": MEET}},
        "data": [row],
    });
    seed(
        &fetcher,
        owned_request.original_url(),
        &serde_json::to_vec(&owned)?,
    )?;
    seed(&fetcher, &reference.url, &raw_body(&url)?)?;
    let context = AdapterContext {
        capture: CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("provider fixture school year")?,
        observed_on: "2026-09-20".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7)
            .ok_or("provider fixture as-of date")?,
        recording: None,
    };
    Ok(collect_result_sets(
        &context,
        &ResultSetOptions {
            urls: vec![ResultSetRequest {
                url: reference.url,
                jurisdiction: UsJurisdiction::Wisconsin,
            }],
        },
    )
    .await?)
}

fn raw_body(url: &str) -> TestResult<Vec<u8>> {
    let header = format!(
        "  Pl {:<26} {:<3} {:<30}{:>15}",
        "Name", "Yr", "Team", "Mark"
    );
    let row = format!(
        "   1 {:<26} {:<3} {:<30}{:>15}",
        "Relay Only Runner", "11", "Catalog School", "51.20"
    );
    Ok(format!(
        r#"<script type="application/ld+json">{{"@context":"https://schema.org","@type":"SportsEvent","name":"Synthetic Wisconsin Meet","description":"Wholly synthetic relay split fixture","startDate":"2026-04-17","endDate":"2026-04-17","homeTeam":"Catalog School","sport":"Outdoor Track & Field","location":{{"@type":"Place","name":"Synthetic Venue","address":{{"@type":"PostalAddress","addressLocality":"Madison","addressRegion":"WI"}}}},"url":"{url}"}}</script><link rel="canonical" href="{url}"><pre>Girls {SPLIT_LABEL} Finals\n{header}\n{row}\n</pre>"#
    )
    .into_bytes())
}

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult {
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("GET\u{1f}{url}\u{1f}").as_bytes())
    );
    let key = digest.get(..32).ok_or("provider fixture cache key")?;
    let metadata = json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body)),
        "bytes": body.len(),
        "fetched_at": FETCHED_AT,
        "content_type": null
    });
    std::fs::create_dir_all(fetcher.cache_dir())?;
    std::fs::write(fetcher.cache_dir().join(format!("{key}.body")), body)?;
    std::fs::write(
        fetcher.cache_dir().join(format!("{key}.meta.json")),
        serde_json::to_vec(&metadata)?,
    )?;
    Ok(())
}
