use super::{dublin, FixtureRun};
use crate::ohsaa::tests::{
    fixture_ad_dublin, fixture_ad_malformed, fixture_sports_dublin, fixture_sports_malformed,
};

#[test]
fn malformed_refused_or_wrong_owner_ad_is_partial_not_successful_absence(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let bodies = [
                fixture_ad_malformed().to_string(),
                "<html><body>No athletic director published</body></html>".to_string(),
                "<html><body>Access denied; please sign in.</body></html>".to_string(),
                fixture_ad_dublin().replace("DUBLIN COFFMAN (474)", "CENTERVILLE (336)"),
                fixture_ad_dublin().replace("Athletic Director:", "Department:"),
                fixture_ad_dublin().replace("Duane Sheldon", ""),
                fixture_ad_dublin().replacen("</td>", "", 1),
            ];
            for body in bodies {
                let run = FixtureRun::new(Some(&body))?;
                let report = run.run("2026-10-02", None).await?;
                check!(eq; (report.rows, report.errors, report.with_email), (1, 1, 4));
                check!(eq; run.counts()?, (1, 4, 1));
                check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
                let outcome = run.outcome()?;
                check!(eq; outcome["state"], "partial");
                check!(eq; outcome["pending_pages"], serde_json::json!([dublin().ad_url()]));
                check!(eq; outcome["failure"]["kind"], "invalid_page");
                check!(eq;
                    outcome["failure"]["capture"]["capture_url"],
                    dublin().ad_url()
                );
                let replay = run.run("2026-10-03", None).await?;
                check!(eq; (replay.rows, replay.errors, replay.with_email), (0, 1, 0));
                check!(eq; replay.requests, 0);
                check!(eq; run.counts()?, (1, 4, 1));
                check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
            }
            Ok(())
        })
}

#[test]
fn malformed_refused_or_wrong_owner_sports_cannot_stamp_complete_school(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let bodies = [
                fixture_sports_malformed().to_string(),
                "<html><body>Access denied; please sign in.</body></html>".to_string(),
                fixture_sports_dublin().replace("DUBLIN COFFMAN (474)", "CENTERVILLE (336)"),
                fixture_sports_dublin().replace("Head Girls Coach", "Student Contacts"),
                fixture_sports_dublin().replacen("</tr>", "", 1),
            ];
            for body in bodies {
                let run = FixtureRun::new_pages(Some(&body), Some(fixture_ad_dublin()))?;
                let report = run.run("2026-10-02", None).await?;
                check!(eq; (report.rows, report.errors, report.with_email), (0, 1, 0));
                check!(eq; run.counts()?, (0, 0, 0));
                check!(run.store.journal_keys("ohsaa_schools")?.is_empty());
                let outcome = run.outcome()?;
                check!(eq; outcome["state"], "failed");
                check!(eq;
                    outcome["pending_pages"],
                    serde_json::json!([dublin().sports_url(), dublin().ad_url()])
                );
                check!(eq; outcome["failure"]["kind"], "invalid_page");
                check!(eq;
                    outcome["failure"]["capture"]["capture_url"],
                    dublin().sports_url()
                );
            }
            Ok(())
        })
}

#[test]
fn repaired_ad_capture_finishes_a_cached_http200_publisher_error(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(Some(fixture_ad_malformed()))?;
            let failed = run.run("2026-10-02", None).await?;
            check!(eq; (failed.rows, failed.errors), (1, 1));
            check!(eq; run.counts()?, (1, 4, 1));
            run.seed(
                &dublin().ad_url(),
                fixture_ad_dublin(),
                "2026-10-03T11:00:00Z",
            )?;
            let repaired = run.run("2026-10-03", None).await?;
            check!(eq;
                (repaired.rows, repaired.errors, repaired.with_email),
                (0, 0, 1)
            );
            check!(eq; run.counts()?, (1, 5, 1));
            check!(eq; run.outcome()?["state"], "complete");
            check!(eq; run.store.journal_payloads("ohsaa_schools")?.len(), 1);
            Ok(())
        })
}

#[test]
fn refused_ad_after_prior_success_keeps_history_but_marks_current_work_partial(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(Some(fixture_ad_dublin()))?;
            run.run("2026-10-02", None).await?;
            let receipts = run.store.journal_payloads("ohsaa_schools")?;
            run.seed(
                &dublin().ad_url(),
                fixture_ad_malformed(),
                "2026-10-03T11:00:00Z",
            )?;
            let failed = run.run("2026-10-03", None).await?;
            check!(eq; (failed.rows, failed.errors, failed.with_email), (0, 1, 0));
            check!(eq; run.counts()?, (1, 5, 1));
            check!(eq; run.store.journal_payloads("ohsaa_schools")?, receipts);
            check!(eq; run.outcome()?["state"], "partial");
            check!(eq; run.outcome()?["pending_pages"], serde_json::json!([dublin().ad_url()]));
            Ok(())
        })
}

#[test]
fn structured_published_ad_absence_is_not_a_person_or_inferred_mailbox(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for placeholder in ["N/A", "TBA"] {
                let body = super::structured_absent_ad().replace("N/A", placeholder);
                let run = FixtureRun::new(Some(&body))?;
                let report = run.run("2026-10-02", None).await?;
                check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 4));
                check!(eq; run.counts()?, (1, 4, 1));
                check!(eq; run.outcome()?["state"], "complete");
                check!(eq; run.store.journal_payloads("ohsaa_schools")?.len(), 1);
            }
            Ok(())
        })
}
