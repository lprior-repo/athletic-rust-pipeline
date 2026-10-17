use super::branch_support as support;
use super::*;
use census_domain::model::{
    CanonicalPerformance, CanonicalSchool, CentiMetres, ExactSeconds, HistoryWindow, Mark,
    Revision, SchoolYear, SourceIdentity, SourceNamespace, SourceObservation,
};
use census_domain::UsJurisdiction;
use census_store::Table;
use sha2::{Digest, Sha256};

const RESULTS: &str = "https://al.milesplit.com/meets/725218/results";
const RAW: &str = "https://al.milesplit.com/meets/725218/results/1266814/raw";
const AT: &str = "2026-10-01T23:44:16Z";
const BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/milesplit/owned/fixtures/troy_725218.json"
));
const RAW_BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"
));

#[tokio::test(flavor = "current_thread")]
async fn controlled_al_projection_nonempty_consumption_survives_lost_ack_same_key_replay(
) -> TestResult {
    let provenance: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../census-crawl/src/milesplit/owned/fixtures/troy_725218.meta.json"
    )))?;
    check!(eq; provenance["fixture_kind"], "selected_source_evidence_projection_not_verbatim_capture");
    check!(eq; provenance["source_fetched_at"], AT);
    check!(provenance["source_digest"] != format!("{:x}", Sha256::digest(BODY)));
    let fixture = controlled_fixture()?;
    let prepared = fixture::seed_prepared_body(&fixture, RESULTS, BODY, AT)?;
    support::publish_exact(&fixture, &prepared)?;
    check!(eq; fixture.store.walk_table(Table::SourceObservations)?.rows, 0);
    check!(eq; fixture.store.walk_table(Table::Performances)?.rows, 0);
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let result = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let plan = protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        support::plan(&plan, &prepared, 1)?;
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let original = protocol::leave_unacknowledged(&mut journal, &frames, "results-v2-execute")?;
        let truth = support::truth(f, &prepared, 1)?;
        support::retained(&truth, &plan)?;
        support::note(&original, "retained", &truth.receipt)?;
        check!(eq; original.per_source.first().ok_or("original source")?.rows, Some(3));
        let rows = retained_rows(f)?;
        let before = durable_snapshot(f)?;
        let receipts = projection_receipts(f)?;
        check!(eq; f.fetcher.stats().await.physical_requests(), 0);
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let bytes = protocol::proposal(&frames)?.bytes(14)?.to_vec();
        let replay = protocol::acknowledge_reexecution(&mut journal, &frames)?;
        support::note(&replay, "retained", &truth.receipt)?;
        check!(eq; replay.per_source.first().ok_or("replayed source")?.rows, Some(3));
        support::unchanged(f, &prepared, 1, &truth)?;
        support::no_successor(f, &prepared, 1)?;
        check!(eq; retained_rows(f)?, rows);
        check!(eq; projection_receipts(f)?, receipts);
        check!(eq; durable_snapshot(f)?, before);
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        endpoint::require_replay_only(&frames)?;
        endpoint::require_saved_outcome(&frames, f, &replay)?;
        check!(eq; endpoint::output(&frames)?, bytes);
        check!(eq; retained_rows(f)?, rows);
        check!(eq; projection_receipts(f)?, receipts);
        check!(eq; durable_snapshot(f)?, before);
        check!(eq; f.fetcher.stats().await.physical_requests(), 0);
        super::super::readmit_same_key(f).await?;
        Ok(())
    })
    .await;
    super::super::finish(&fixture, result, 3, resources).await
}

fn controlled_fixture() -> TestResult<Fixture> {
    let request = crate::restate_services::wire::JurisdictionRequest {
        jurisdiction: UsJurisdiction::Alabama,
        season: SchoolYear::new(2026).ok_or("controlled season")?,
        revision: Revision(1),
        history: HistoryWindow::new(2026, 2026, "2026-10-14")?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: None,
        authorized_hosts: vec!["al.milesplit.com".to_owned()],
        source_parallelism: 1,
    };
    let meet = census_domain::model::SourceMeetRef {
        jurisdiction: UsJurisdiction::Alabama,
        season: "outdoor".to_owned(),
        name: "Troy Invitational #2".to_owned(),
        date: Some("2026-03-27".to_owned()),
        venue: String::new(),
        results_url: RESULTS.to_owned(),
        ..super::super::fixture::meet()
    };
    let fixture = Fixture::with_inputs(CaptureProtocolVersion::V2, request, meet)?;
    crate::capture_cache::seed(fixture.fetcher.cache_dir(), RAW, RAW_BODY, AT, &[])?;
    for (name, id) in [
        ("Controlled school 38332", "38332"),
        ("Controlled school 4912", "4912"),
    ] {
        let mut school = CanonicalSchool::new(UsJurisdiction::Alabama, name, name, None).0;
        school
            .source_identities
            .push(SourceIdentity::new(SourceNamespace::MilesplitSchool, id));
        fixture.store.append(Table::Schools, &school)?;
    }
    Ok(fixture)
}

fn retained_rows(fixture: &Fixture) -> TestResult<serde_json::Value> {
    let view = fixture.store.snapshot();
    let mut observations = Vec::new();
    view.for_each_observation(Table::SourceObservations, |row: SourceObservation| {
        observations.push(row);
        Ok(())
    })?;
    let mut performances = Vec::new();
    view.for_each_observation(Table::Performances, |row: CanonicalPerformance| {
        performances.push(row);
        Ok(())
    })?;
    check!(eq; observations.len(), 3);
    check!(eq; performances.len(), 3);
    require_exact_marks(&performances)?;
    check!(observations.iter().all(|row| row.observed_on() == AT));
    let expected = std::collections::BTreeSet::from([
        "milesplit_result:201782263",
        "milesplit_result:201782277",
        "milesplit_result:201782806",
    ]);
    check!(eq; performances.iter().map(|row| row.source_key.as_str()).collect::<std::collections::BTreeSet<_>>(), expected);
    let source_keys = observations
        .iter()
        .map(|row| match row {
            SourceObservation::Athlete(row) => Ok(row.source_row_key.as_str()),
            SourceObservation::School(_) => Err("unexpected school observation"),
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    check!(eq; source_keys, expected);
    check!(performances.iter().all(|row| row.date == "2026-03-27"
        && row
            .evidence
            .iter()
            .all(|evidence| evidence.observed_on == AT)));
    observations.sort_by_key(|row| row.id().to_owned());
    performances.sort_by(|left, right| left.source_key.cmp(&right.source_key));
    Ok(serde_json::json!({"observations": observations, "performances": performances}))
}

fn durable_snapshot(fixture: &Fixture) -> TestResult<serde_json::Value> {
    Ok(serde_json::json!({
        "acquisition": acquisition(fixture)?,
        "sequence": fixture.store.snapshot().sequence(),
        "receipts": fixture.store.receipt_count()?,
    }))
}

fn projection_receipts(fixture: &Fixture) -> TestResult<serde_json::Value> {
    let phase = "milesplit_result_sets_v6";
    let payloads = fixture.store.journal_payloads(phase)?;
    let projected = payloads
        .iter()
        .find(|row| row["disposition"] == "projection_applied")
        .ok_or("nonempty projection receipt absent")?;
    check!(eq; projected["projected_rows"], 3);
    check!(eq; projected["jurisdiction"], "AL");
    check!(eq; projected["capture"]["content_digest"], format!("{:x}", Sha256::digest(BODY)));
    check!(eq; projected["raw_metadata_capture"]["content_digest"], format!("{:x}", Sha256::digest(RAW_BODY)));
    check!(eq; projected["source_completeness"], "unknown");
    check!(eq; projected["canonical_identity_accepted"], false);
    check!(eq; projected["census_sealed"], false);
    let mut keys: Vec<_> = fixture.store.journal_keys(phase)?.into_iter().collect();
    keys.sort();
    let values = keys
        .into_iter()
        .map(|key| Ok((key.clone(), fixture.store.journal_payload(phase, &key)?)))
        .collect::<TestResult<Vec<_>>>()?;
    Ok(serde_json::to_value(values)?)
}

fn require_exact_marks(rows: &[CanonicalPerformance]) -> TestResult {
    for (source, expected) in [
        (
            "milesplit_result:201782263",
            Mark::FieldImperial {
                feet_mark: "13-9".to_owned(),
                metres: CentiMetres::new(419),
            },
        ),
        (
            "milesplit_result:201782277",
            Mark::FieldImperial {
                feet_mark: "30-7".to_owned(),
                metres: CentiMetres::new(932),
            },
        ),
        (
            "milesplit_result:201782806",
            Mark::TimeSeconds(ExactSeconds::parse("12.40")?),
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.source_key == source)
            .ok_or("expected controlled performance absent")?;
        check!(eq; row.mark, expected);
    }
    Ok(())
}
