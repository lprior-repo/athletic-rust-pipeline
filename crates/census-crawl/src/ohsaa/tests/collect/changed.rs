use super::{dublin, FixtureRun};
use crate::net::cache::content_digest;
use crate::ohsaa::tests::{fixture_ad_dublin, fixture_sports_dublin};
use census_domain::model::CanonicalCoach;
use census_store::Table;

#[test]
fn changed_sports_after_completed_capture_retains_new_and_old_publisher_facts(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let run = FixtureRun::new(Some(fixture_ad_dublin()))?;
    run.run("2026-10-02", None).await?;
    let changed = fixture_sports_dublin().replace("Joe DePalma", "Jordan Coach");
    run.seed(&dublin().sports_url(), &changed, "2026-10-03T10:00:00Z")?;
    let report = run.run("2026-10-03", None).await?;
    check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 4));
    check!(eq; run.counts()?, (2, 9, 2));
    let coaches: Vec<CanonicalCoach> = run.store.scan(Table::Coaches)?;
    check!(coaches.iter().any(|coach| coach.name == "Joe DePalma"));
    let current = coaches
        .iter()
        .find(|coach| coach.name == "Jordan Coach")
        .ok_or_else(|| anyhow::anyhow!("changed coach absent"))?;
    check!(eq; current.evidence.first().ok_or("changed capture evidence")?.observed_on, "2026-10-03T10:00:00Z");
    let receipts = run.store.journal_payloads("ohsaa_schools")?;
    check!(eq; receipts.len(), 2);
    check!(receipts
        .iter()
        .any(|receipt| receipt["sports_capture"]["sha256"] == content_digest(changed.as_bytes())));
    run.run("2026-10-04", None).await?;
    check!(eq; run.counts()?, (2, 9, 2));
    check!(eq; run.store.journal_payloads("ohsaa_schools")?, receipts);
    Ok(())
    })
}

#[test]
fn changed_ad_after_completed_capture_does_not_reappend_sport_facts(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(Some(fixture_ad_dublin()))?;
            run.run("2026-10-02", None).await?;
            let changed = fixture_ad_dublin().replace("Duane Sheldon", "Taylor Director");
            run.seed(&dublin().ad_url(), &changed, "2026-10-03T11:00:00Z")?;
            let report = run.run("2026-10-03", None).await?;
            check!(eq; (report.rows, report.errors, report.with_email), (0, 0, 1));
            check!(eq; run.counts()?, (1, 6, 1));
            let coaches: Vec<CanonicalCoach> = run.store.scan(Table::Coaches)?;
            check!(coaches.iter().any(|coach| coach.name == "Duane Sheldon"));
            check!(coaches.iter().any(|coach| coach.name == "Taylor Director"));
            let receipts = run.store.journal_payloads("ohsaa_schools")?;
            check!(eq; receipts.len(), 2);
            check!(receipts.iter().any(
                |receipt| receipt["ad_capture"]["sha256"] == content_digest(changed.as_bytes())
            ));
            run.run("2026-10-04", None).await?;
            check!(eq; run.counts()?, (1, 6, 1));
            check!(eq; run.store.journal_payloads("ohsaa_schools")?, receipts);
            Ok(())
        })
}

#[test]
fn historical_school_only_receipt_is_preserved_but_does_not_suppress_capture(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(Some(fixture_ad_dublin()))?;
            let mut legacy = run.store.write_batch();
            legacy.journal_done(
                "ohsaa_schools",
                "OH:474",
                &serde_json::json!({"legacy": true}),
            )?;
            legacy.commit()?;
            let report = run.run("2026-10-02", None).await?;
            check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 5));
            check!(eq; run.counts()?, (1, 5, 1));
            let keys = run.store.journal_keys("ohsaa_schools")?;
            check!(keys.contains("OH:474"));
            let receipts = run.store.journal_payloads("ohsaa_schools")?;
            check!(receipts.iter().any(|receipt| receipt["legacy"] == true));
            check!(receipts
                .iter()
                .any(|receipt| receipt["state"] == "complete"));
            Ok(())
        })
}
