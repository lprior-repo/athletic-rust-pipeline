use super::super::directory::OWNED_AND_FOREIGN;
use census_domain::model::CanonicalCoach;

#[test]
fn collection_and_replay_never_publish_later_foreign_table_rows() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let run = super::FixtureRun::new_pages(Some(OWNED_AND_FOREIGN), Some(super::super::fixture_ad_dublin()))?;
        let report = run.run("2026-10-02", None).await?;
        check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 2));
        let coaches: Vec<CanonicalCoach> = run.store.scan(census_store::Table::Coaches)?;
        let names: std::collections::BTreeSet<_> = coaches.iter().map(|coach| coach.name.as_str()).collect();
        check!(eq; names, ["Owned Coach", "Duane Sheldon"].into_iter().collect());
        check!(eq; run.outcome()?["state"], "complete");
        let replay = run.run("2026-10-03", None).await?;
        check!(eq; (replay.rows, replay.errors, replay.with_email), (0, 0, 0));
        check!(eq; run.store.scan::<CanonicalCoach>(census_store::Table::Coaches)?, coaches);
        Ok(())
    })
}
