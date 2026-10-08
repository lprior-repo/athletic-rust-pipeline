use super::{dublin, FixtureRun, CAPTURES};
use crate::ohsaa::tests::{fixture_ad_dublin, SPORTS_FETCHED};
use crate::Recording;
use census_domain::model::CanonicalCoach;
use census_store::Table;

#[test]
fn identical_partial_replay_keeps_ad_unfinished_without_duplicate_physical_facts(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(None)?;
            let first = run.run("2026-10-02", None).await?;
            check!(eq; (first.rows, first.errors, first.with_email), (1, 1, 4));
            check!(eq; first.requests, 0);
            check!(eq; run.counts()?, (1, 4, 1));
            check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
            let captured: Vec<CanonicalCoach> = run.store.scan(Table::Coaches)?;
            let outcome = run.outcome()?;
            check!(eq; outcome["state"], "partial");
            check!(eq; outcome["pending_pages"], serde_json::json!([dublin().ad_url()]));
            check!(eq; outcome["sports_capture"]["acquired_at"], SPORTS_FETCHED);
            check!(eq; outcome["failure"]["kind"], "fetch");
            let key = run
                .store
                .journal_keys(CAPTURES)?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("sports marker"))?;
            let payload = run
                .store
                .journal_payloads(CAPTURES)?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("sports marker payload"))?;
            let mut restamped = run.store.write_batch();
            restamped.journal_done(CAPTURES, &key, &payload)?;
            restamped.commit()?;
            let replay = run.run("2026-10-03", None).await?;
            check!(eq; (replay.rows, replay.errors, replay.with_email), (0, 1, 0));
            check!(eq; replay.requests, 0);
            check!(eq; run.counts()?, (1, 4, 1));
            check!(eq; run.store.scan::<CanonicalCoach>(Table::Coaches)?, captured);
            check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
            check!(eq; run.outcome()?["state"], "partial");
            check!(eq; run.outcome()?["evaluated_on"], "2026-10-03");
            check!(eq; run.store.journal_keys(CAPTURES)?.len(), 1);
            Ok(())
        })
}

#[test]
fn recovered_ad_adds_only_director_and_finishes_existing_school(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(None)?;
            let partial = run.run("2026-10-02", None).await?;
            check!(eq; (partial.rows, partial.errors), (1, 1));
            run.seed(
                &dublin().ad_url(),
                fixture_ad_dublin(),
                super::super::AD_FETCHED,
            )?;
            let recovered = run.run("2026-10-03", None).await?;
            check!(eq;
                (recovered.rows, recovered.errors, recovered.with_email),
                (0, 0, 1)
            );
            check!(eq; recovered.requests, 0);
            check!(eq; run.counts()?, (1, 5, 1));
            check!(eq; run.outcome()?["state"], "complete");
            check!(eq; run.outcome()?["pending_pages"], serde_json::json!([]));
            let receipts = run.store.journal_payloads("ohsaa_schools")?;
            check!(eq; receipts.len(), 1);
            check!(eq; run.store.journal_keys(CAPTURES)?.len(), 2);
            let replay = run.run("2026-10-04", None).await?;
            check!(eq; (replay.rows, replay.errors, replay.with_email), (0, 0, 0));
            check!(eq; replay.requests, 0);
            check!(eq; run.counts()?, (1, 5, 1));
            check!(eq; run.store.journal_payloads("ohsaa_schools")?, receipts);
            Ok(())
        })
}

#[test]
fn partial_recording_leaves_store_untouched_and_applied_replay_unfinished(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(None)?;
            let recording = Recording::new();
            let first = run.run("2026-10-02", Some(&recording)).await?;
            check!(eq; (first.rows, first.errors), (1, 1));
            check!(eq; run.counts()?, (0, 0, 0));
            check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
            let captured = recording.drain();
            check!(!captured
                .journal
                .iter()
                .any(|entry| entry.phase == "ohsaa_schools"));
            let mut batch = run.store.write_batch();
            captured
                .rows
                .iter()
                .try_for_each(|rows| batch.append_many(rows.table, &rows.rows))?;
            captured.journal.iter().try_for_each(|entry| {
                batch.journal_done(&entry.phase, &entry.key, &entry.payload)
            })?;
            batch.commit()?;
            check!(eq; run.counts()?, (1, 4, 1));
            let replay = run.run("2026-10-03", Some(&recording)).await?;
            check!(eq; (replay.rows, replay.errors), (0, 1));
            let repeated = recording.drain();
            check!(repeated.rows.is_empty());
            check!(eq; repeated.journal.len(), 1);
            let receipt = repeated.journal.first().ok_or("partial replay receipt")?;
            check!(eq; receipt.phase, super::OUTCOMES);
            check!(eq; receipt.payload["state"], "partial");
            check!(eq; run.counts()?, (1, 4, 1));
            Ok(())
        })
}
