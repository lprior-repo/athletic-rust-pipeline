use super::*;
use crate::net::capture_fixture::{serve, Fixture, Reply, BODY, CAPTURED};
use std::sync::Arc;
use tokio::net::TcpListener;

type DurableResult<T = ()> = crate::net::capture_fixture::TestResult<T>;

#[tokio::test]
async fn durable_active_meet_reuses_one_grant_for_multiple_result_sets_and_replays() -> DurableResult
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let owner = fixture.context_owner(2026, false)?;
    let ctx = fixture.context(&owner, false)?;
    let reference = fixture.reference()?;
    let mut document: serde_json::Value = serde_json::from_slice(BODY)?;
    let row = document
        .get_mut("data")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|rows| rows.get_mut(1))
        .ok_or("second fixture row")?;
    *row.get_mut("meetResultsId").ok_or("result-set field")? = json!("1266815");
    let body = serde_json::to_vec(&document)?;
    let fetch = async {
        let mut run = Run::new(ProviderSchools::default());
        run.acquire_owned(&ctx, &reference).await?;
        let mut second = reference.clone();
        second.rsid = "1266815".into();
        second.url = "https://mi.milesplit.com/meets/725218/results/1266815/raw".into();
        run.acquire_owned(&ctx, &second).await?;
        let acquired = acquired(&run.owned)?;
        let first_rows = acquired
            .result_set("1266814", ctx.performance_as_of)
            .ok_or("first result set")?;
        let second_rows = acquired
            .result_set("1266815", ctx.performance_as_of)
            .ok_or("second result set")?;
        check!(eq; first_rows.indices.len(), 2);
        check!(eq; second_rows.indices.len(), 1);
        let first_ids = first_rows
            .indices
            .iter()
            .map(|index| {
                first_rows
                    .page
                    .rows
                    .get(*index)
                    .map(|row| row.result_id)
                    .ok_or("first row index")
            })
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        let second_ids = second_rows
            .indices
            .iter()
            .map(|index| {
                second_rows
                    .page
                    .rows
                    .get(*index)
                    .map(|row| row.result_id)
                    .ok_or("second row index")
            })
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        check!(eq; first_ids, std::collections::BTreeSet::from([201782263, 201782806]));
        check!(eq; second_ids, std::collections::BTreeSet::from([201782277]));
        check!(eq; acquired.outcome.capture().body.len(), 0);
        check!(eq; acquired.outcome.capture().fetched_at, CAPTURED);
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            Fixture::occurrence(&acquired.outcome)?.clone(),
        )
    };
    let ((), original) = tokio::try_join!(
        serve(listener, [Reply::ok(&body)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    let owner = fixture.context_owner(2026, false)?;
    let replay_ctx = fixture.context(&owner, false)?;
    let mut replay = Run::new(ProviderSchools::default());
    replay.acquire_owned(&replay_ctx, &reference).await?;
    check!(eq; Fixture::occurrence(&acquired(&replay.owned)?.outcome)?, &original);
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn prepared_recovery_enters_acquired_meet_and_real_parser_without_http() -> DurableResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    fixture.stop_after_archive();
    let fetch = async {
        let result = fixture.acquire(2026, false).await?;
        check!(matches!(result, Err(ref error) if crate::net::capture_fixture::injected(error)));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    fixture.resume_capture();
    let owner = fixture.context_owner(2026, false)?;
    let ctx = fixture.context(&owner, false)?;
    let mut run = Run::new(ProviderSchools::default());
    run.acquire_owned(&ctx, &fixture.reference()?).await?;
    let acquired = acquired(&run.owned)?;
    check!(
        matches!(acquired.outcome.verdict(), crate::milesplit::OwnedMeetVerdict::Parsed(page) if !page.rows.is_empty())
    );
    check!(eq; Fixture::occurrence(&acquired.outcome)?.attempt().ordinal().get(), 1);
    check!(eq; acquired.outcome.capture().fetched_at, CAPTURED);
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn durable_run_request_projects_then_reopens_and_replays_the_same_capture() -> DurableResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let schools = [
        school("Spann provider school", "38332"),
        school("Charles", "4912"),
    ];
    for school in &schools {
        fixture.store.append(Table::Schools, school)?;
    }
    let reference = fixture.reference()?;
    let raw = std::str::from_utf8(FEMALE_RAW)?.replace("al.milesplit.com", "mi.milesplit.com");
    let key = Fetcher::key_for("GET", &reference.url, "");
    let (body, metadata) = fixture.fetcher.cache_paths(&key);
    crate::net::cache::write_cache(
        &body,
        &metadata,
        raw.as_bytes(),
        &crate::net::cache::CacheMeta {
            url: reference.url.clone(),
            response_url: Some(reference.url.clone()),
            method: "GET".into(),
            status: 200,
            content_digest: crate::net::cache::content_digest(raw.as_bytes()),
            bytes: raw.len(),
            fetched_at: CAPTURED.into(),
            ..crate::net::cache::CacheMeta::default()
        },
    )?;
    let request = crate::milesplit::ResultSetRequest {
        url: reference.url,
        jurisdiction: census_domain::UsJurisdiction::Michigan,
    };
    let fetch = async {
        let owner = fixture.context_owner(2026, false)?;
        let ctx = fixture.context(&owner, false)?;
        let mut run = Run::new(ProviderSchools::from_schools(&schools));
        run.request(&ctx, &request, 0).await?;
        check!(run.stats.rows > 0);
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            Fixture::occurrence(&acquired(&run.owned)?.outcome)?.clone(),
        )
    };
    let ((), original) = tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    let before: Vec<CanonicalPerformance> = fixture.store.scan(Table::Performances)?;
    check!(eq; before.len(), 3);
    check!(eq; before.iter().map(|row| row.source_key.as_str()).collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["milesplit_result:201782263", "milesplit_result:201782277", "milesplit_result:201782806"]));
    check!(before.iter().all(|row| row.date == "2026-03-27"));
    let receipts = fixture
        .store
        .journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
    let projection = receipts
        .iter()
        .find(|row| {
            row.get("disposition").and_then(serde_json::Value::as_str) == Some("projection_applied")
        })
        .ok_or("projection receipt")?;
    check!(eq; projection.get("projected_rows").and_then(serde_json::Value::as_u64), Some(3));
    check!(eq; projection.get("meet").and_then(serde_json::Value::as_str), Some("725218"));
    check!(eq; projection.get("rsid").and_then(serde_json::Value::as_str), Some("1266814"));
    let acquisition_roots = fixture
        .store
        .journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?;
    let fixture = fixture.reopen()?;
    let owner = fixture.context_owner(2026, false)?;
    let ctx = fixture.context(&owner, false)?;
    let mut replay = Run::new(ProviderSchools::from_schools(&schools));
    replay.request(&ctx, &request, 0).await?;
    let after: Vec<CanonicalPerformance> = fixture.store.scan(Table::Performances)?;
    check!(eq; serde_json::to_value(&before)?, serde_json::to_value(&after)?);
    check!(eq; fixture.store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?, receipts);
    check!(eq; fixture.store.journal_payloads(census_domain::model::SOURCE_ACQUISITION_PHASE)?, acquisition_roots);
    check!(eq; Fixture::occurrence(&acquired(&replay.owned)?.outcome)?, &original);
    check!(eq; fixture.count(), 1);
    Ok(())
}
