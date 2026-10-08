use super::*;
use crate::milesplit::results::{ResultSetOptions, ResultSetRequest};
use census_domain::UsJurisdiction;
use futures::{StreamExt, TryStreamExt};

pub(super) fn document(
    meet: u64,
    sets: usize,
    rows_per_set: usize,
) -> TestResult<serde_json::Value> {
    let template: serde_json::Value = serde_json::from_slice(TROY)?;
    let template = template
        .get("data")
        .and_then(serde_json::Value::as_array)
        .and_then(|rows| rows.first())
        .ok_or("owned fixture first row")?;
    let count = sets
        .checked_mul(rows_per_set)
        .ok_or("fixture row count overflow")?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)?;
    let rows = (0..sets)
        .flat_map(|set| (0..rows_per_set).map(move |row| (set, row)))
        .try_fold(rows, |mut held, (set, row)| {
            held.push(fixture_row(template, meet, set, row, rows_per_set)?);
            Ok::<_, Box<dyn std::error::Error>>(held)
        })?;
    Ok(json!({"data": rows}))
}

fn fixture_row(
    template: &serde_json::Value,
    meet: u64,
    set: usize,
    row: usize,
    rows_per_set: usize,
) -> TestResult<serde_json::Value> {
    let offset = set
        .checked_mul(rows_per_set)
        .and_then(|offset| offset.checked_add(row))
        .ok_or("fixture row offset overflow")?;
    let id = meet
        .checked_mul(10_000)
        .and_then(|id| id.checked_add(201_782_263))
        .and_then(|id| {
            u64::try_from(offset)
                .ok()
                .and_then(|offset| id.checked_add(offset))
        })
        .ok_or("fixture ID overflow")?;
    let set_id = 1_266_814u64
        .checked_add(u64::try_from(set)?)
        .ok_or("fixture set ID overflow")?;
    let mut value = template.clone();
    value["meetId"] = json!(meet.to_string());
    value["meetResultsId"] = json!(set_id.to_string());
    value["id"] = json!(id.to_string());
    Ok(value)
}

pub(super) fn seed_set(fetcher: &Fetcher, meet: u64, set: u64) -> TestResult<ResultSetRequest> {
    let url = format!("https://al.milesplit.com/meets/{meet}/results/{set}/raw");
    let raw = std::str::from_utf8(FEMALE_RAW)?
        .replace("725218", &meet.to_string())
        .replace("1266814", &set.to_string());
    seed(fetcher, &url, raw.as_bytes())?;
    Ok(ResultSetRequest {
        url,
        jurisdiction: UsJurisdiction::Alabama,
    })
}

fn peak(report: &crate::AdapterReport) -> TestResult<(usize, usize)> {
    let note = report
        .notes
        .iter()
        .find(|note| note.starts_with("result buffers:"))
        .ok_or("buffer report")?;
    let fields: Vec<_> = note.split_whitespace().collect();
    Ok((
        fields
            .get(6)
            .ok_or("window bytes")?
            .trim_end_matches(';')
            .parse()?,
        fields
            .get(11)
            .ok_or("capture bytes")?
            .trim_end_matches(';')
            .parse()?,
    ))
}

pub(super) async fn replay_once(
    store: &Store,
    fetcher: &Fetcher,
    options: &ResultSetOptions,
    expected: usize,
) -> TestResult {
    let before = store.walk_table(Table::Performances)?;
    let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
    let report = crate::milesplit::collect_result_sets(&context(store, fetcher)?, options).await?;
    check!(eq; report.rows, u64::try_from(expected)?);
    check!(eq; store.walk_table(Table::Performances)?, before);
    check!(eq; store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?, receipts);
    Ok(())
}

#[test]
fn ten_hundred_thousand_distinct_captures_have_bounded_collector_retention_and_exact_replay(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let peaks = futures::stream::iter([10usize, 100, 1000])
                .map(Ok::<_, Box<dyn std::error::Error>>)
                .try_fold(Vec::new(), |mut peaks, count| async move {
                    peaks.push(distinct_captures(count).await?);
                    Ok::<_, Box<dyn std::error::Error>>(peaks)
                })
                .await?;
            check!(peaks.iter().all(|(window, capture)| *window
                <= super::super::super::budget::WINDOW_BYTES
                && *capture <= crate::milesplit::owned::MAX_OWNED_BODY_BYTES));
            check!(peaks.windows(2).all(|pair| pair[0] == pair[1]));
            Ok(())
        })
}

async fn distinct_captures(count: usize) -> TestResult<(usize, usize)> {
    let (_dir, store, fetcher, _) = setup()?;
    let urls = (0..count)
        .map(|index| {
            let meet = 725_218u64
                .checked_add(u64::try_from(index)?)
                .ok_or("fixture meet ID overflow")?;
            let request = seed_set(&fetcher, meet, 1_266_814)?;
            let reference = ResultSetRef::parse(&request.url).ok_or("reference")?;
            seed_owned(
                &fetcher,
                &reference,
                &serde_json::to_vec(&document(meet, 1, 1)?)?,
            )?;
            Ok::<_, Box<dyn std::error::Error>>(request)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let options = ResultSetOptions { urls };
    let report =
        crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &options).await?;
    check!(eq; (report.rows, report.errors), (u64::try_from(count)?, 0));
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq; performances.len(), count);
    check!(eq; performances.iter().map(|row| row.source_key.as_str()).collect::<std::collections::BTreeSet<_>>().len(), count);
    check!(performances.iter().all(|row| row
        .source_athlete
        .as_ref()
        .is_some_and(|source| source.id == "14222592")));
    replay_once(&store, &fetcher, &options, count).await?;
    peak(&report)
}

#[test]
fn a_thousand_result_sets_share_one_bounded_active_capture_and_preserve_every_fact() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, &serde_json::to_vec(&document(725218, 1000, 1)?)?)?;
        let urls = (0..1000u64).map(|index| seed_set(&fetcher, 725218, 1_266_814u64.checked_add(index).ok_or("fixture result set ID overflow")?))
            .collect::<TestResult<Vec<_>>>()?;
        let options = ResultSetOptions { urls };
        let report = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &options).await?;
        check!(eq; (report.rows, report.errors), (1000, 0));
        check!(eq; fetcher.stats().await.cache_hits, 1001);
        let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        check!(eq; performances.len(), 1000);
        let mut run = Run::new(ProviderSchools::from_schools(&[school("Spann provider school", "38332")]));
        run.read(&context(&store, &fetcher)?, &reference).await?;
        let Some(ActiveMeet { meet, .. }) = &run.owned else { return Err("active capture".into()); };
        check!(meet.outcome.capture.body.is_empty());
        check!(eq; meet.result_set("1266814", context(&store, &fetcher)?.performance_as_of).ok_or("result set")?.page.rows.len(), 1000);
        replay_once(&store, &fetcher, &options, 1000).await?;
        Ok(())
    })
}
