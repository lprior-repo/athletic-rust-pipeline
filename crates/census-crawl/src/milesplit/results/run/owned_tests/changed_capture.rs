use super::replay::{entities, physical};
use super::*;
use crate::net::cache::content_digest;

mod support;
use support::*;

async fn apply_original(store: &Store, fetcher: &Fetcher, reference: &ResultSetRef) -> TestResult {
    seed_owned(fetcher, reference, TROY)?;
    seed_metadata(fetcher, reference)?;
    let report =
        crate::milesplit::collect_result_sets(&context(store, fetcher)?, &options(reference))
            .await?;
    check!(eq; report.errors, 0, "{:?}", report.notes);
    assert_archived(store, TROY, &content_digest(TROY))?;
    assert_owned_year(store, 201782263, 2027)?;
    assert_canonical_year(store, &content_digest(TROY), 201782263, 2027)?;
    Ok(())
}

async fn assert_identical_replay(
    store: &Store,
    fetcher: &Fetcher,
    reference: &ResultSetRef,
) -> TestResult {
    let before = entities(store)?;
    let before_physical = physical(store)?;
    let before_receipts = projection_receipts(store)?;
    let owned_before = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
    let captures_before = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
    let report =
        crate::milesplit::collect_result_sets(&context(store, fetcher)?, &options(reference))
            .await?;
    check!(eq; report.errors, 0, "{:?}", report.notes);
    check!(eq; entities(store)?, before);
    check!(eq; physical(store)?, before_physical);
    check!(eq; projection_receipts(store)?, before_receipts);
    check!(eq;
        store
            .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)
            ?,
        owned_before
    );
    check!(eq;
        store
            .journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)
            ?,
        captures_before
    );
    Ok(())
}

#[test]
fn completed_troy_set_recaptured_with_year_2028_retains_new_canonical_review_evidence() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            apply_original(&store, &fetcher, &reference).await?;
            assert_identical_replay(&store, &fetcher, &reference).await?;
            let prior = projection_receipts(&store)?;
            let keys = store.journal_keys(crate::milesplit::RESULT_SET_PHASE)?;
            let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
            let row = document["data"]
                .as_array_mut()
                .ok_or("captured source rows")?
                .iter_mut()
                .find(|row| row["id"] == 201782263)
                .ok_or("exact source result")?;
            check!(eq; row["athleteId"], "14222592");
            check!(eq; row["gradYear"], "2027");
            row["gradYear"] = json!("2028");
            let changed = serde_json::to_vec(&document)?;
            let url = crate::milesplit::fetch::owned_meet_url(&reference)?;
            let digest = seed_fresh(&fetcher, &url, &changed)?;
            check!(ne; digest, content_digest(TROY));
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.errors, 0, "{:?}", report.notes);
            assert_archived(&store, TROY, &content_digest(TROY))?;
            assert_archived(&store, &changed, &digest)?;
            assert_owned_year(&store, 201782263, 2027)?;
            assert_owned_year(&store, 201782263, 2028)?;
            assert_canonical_year(&store, &content_digest(TROY), 201782263, 2027)?;
            assert_canonical_year(&store, &digest, 201782263, 2028)?;
            assert_separate_projection(&store, &keys, &prior, &digest)?;
            assert_identical_replay(&store, &fetcher, &reference).await?;
            Ok(())
        })
}

#[test]
fn completed_troy_set_recaptured_with_new_result_retains_the_source_owned_performance() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            apply_original(&store, &fetcher, &reference).await?;
            let prior = projection_receipts(&store)?;
            let keys = store.journal_keys(crate::milesplit::RESULT_SET_PHASE)?;
            let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
            let rows = document["data"].as_array_mut().ok_or("provider rows")?;
            let mut added = rows
                .iter()
                .find(|row| row["id"] == 201782263)
                .ok_or("owned source record")?
                .clone();
            added["id"] = json!(201782264);
            added["mark"] = json!("14-0");
            added["units"] = json!("168000");
            rows.push(added);
            let changed = serde_json::to_vec(&document)?;
            let url = crate::milesplit::fetch::owned_meet_url(&reference)?;
            let digest = seed_fresh(&fetcher, &url, &changed)?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.errors, 0, "{:?}", report.notes);
            assert_archived(&store, TROY, &content_digest(TROY))?;
            assert_archived(&store, &changed, &digest)?;
            assert_owned_year(&store, 201782264, 2027)?;
            assert_canonical_year(&store, &digest, 201782264, 2027)?;
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let added = performances
                .iter()
                .find(|row| row.source_key == "milesplit_result:201782264")
                .ok_or("new source result cannot be suppressed as completed-set replay")?;
            check!(eq;
                added.mark,
                Mark::FieldImperial {
                    feet_mark: "14-0".into(),
                    metres: census_domain::model::CentiMetres::new(427),
                }
            );
            assert_separate_projection(&store, &keys, &prior, &digest)?;
            assert_identical_replay(&store, &fetcher, &reference).await?;
            Ok(())
        })
}

#[test]
fn completed_troy_set_with_changed_raw_metadata_retains_both_meet_dates() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            apply_original(&store, &fetcher, &reference).await?;
            let prior = projection_receipts(&store)?;
            let keys = store.journal_keys(crate::milesplit::RESULT_SET_PHASE)?;
            let raw = std::str::from_utf8(FEMALE_RAW)?;
            let changed = raw.replace("2026-03-27", "2025-03-27");
            let digest = seed_fresh(&fetcher, &reference.url, changed.as_bytes())?;
            check!(ne; digest, content_digest(FEMALE_RAW));
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.errors, 0, "{:?}", report.notes);
            assert_separate_projection(&store, &keys, &prior, &content_digest(TROY))?;
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let notes: Vec<_> = performances
                .iter()
                .filter(|row| row.source_key == "milesplit_result:201782263")
                .flat_map(|row| evidence_notes(&row.evidence))
                .collect();
            for date in ["2026-03-27", "2025-03-27"] {
                check!(
                    notes.iter().any(|note| {
                        note["result_id"] == 201782263
                            && note["source_athlete"]["id"] == "14222592"
                            && note["published_grad_year"] == 2027
                            && note["meet_date"] == date
                            && note["sha256"] == content_digest(TROY)
                    }),
                    "canonical result evidence must retain published meet date {date}"
                );
            }
            let meets: Vec<census_domain::model::CanonicalMeet> = store.scan(Table::Meets)?;
            for date in ["2026-03-27", "2025-03-27"] {
                check!(
                    meets.iter().any(|meet| {
                        meet.date == date
                            && meet
                                .source_identities
                                .iter()
                                .any(|owner| owner.id == "725218")
                    }),
                    "changed metadata must remain source-owned at canonical meet date {date}"
                );
            }
            assert_identical_replay(&store, &fetcher, &reference).await?;
            Ok(())
        })
}
