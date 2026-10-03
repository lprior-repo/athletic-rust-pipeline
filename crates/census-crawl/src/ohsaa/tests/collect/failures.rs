use super::{FixtureRun, CAPTURES};

#[test]
fn ad_fetch_failure_preserves_sport_facts_and_reports_an_unfinished_obligation(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(None)?;
            let report = run.run("2026-10-02", None).await?;
            check!(eq; (report.rows, report.errors, report.with_email), (1, 1, 4));
            check!(eq; run.counts()?, (1, 4, 1));
            check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
            let outcome = run.outcome()?;
            check!(eq; outcome["state"], "partial");
            check!(eq; outcome["pending_pages"], serde_json::json!(["ad"]));
            check!(eq; outcome["failure"]["kind"], "fetch");
            check!(outcome["failure"]["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("offline and not cached:")));
            check!(eq;
                report
                    .notes
                    .iter()
                    .filter(|note| note.contains("unfinished obligation"))
                    .count(),
                1
            );
            check!(report
                .notes
                .iter()
                .any(|note| note.contains("1 fetch_failures")));
            check!(eq; run.store.journal_keys(CAPTURES)?.len(), 1);
            Ok(())
        })
}

#[test]
fn sports_fetch_failure_is_visible_and_does_not_complete_or_create_facts(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new_pages(None, Some(super::super::fixture_ad_dublin()))?;
            let report = run.run("2026-10-02", None).await?;
            check!(eq; (report.rows, report.errors, report.with_email), (0, 1, 0));
            check!(eq; report.requests, 0);
            check!(eq; run.counts()?, (0, 0, 0));
            check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
            check!(run.store.journal_keys(CAPTURES)?.is_empty());
            let outcome = run.outcome()?;
            check!(eq; outcome["state"], "failed");
            check!(eq; outcome["failed_page"], "sports");
            check!(eq;
                outcome["pending_pages"],
                serde_json::json!(["sports", "ad"])
            );
            check!(eq; outcome["failure"]["kind"], "fetch");
            check!(report
                .notes
                .iter()
                .any(|note| note.contains("1 fetch_failures")));
            let replay = run.run("2026-10-03", None).await?;
            check!(eq; (replay.rows, replay.errors), (0, 1));
            check!(eq; run.counts()?, (0, 0, 0));
            check!(eq; run.outcome()?["state"], "failed");
            Ok(())
        })
}
