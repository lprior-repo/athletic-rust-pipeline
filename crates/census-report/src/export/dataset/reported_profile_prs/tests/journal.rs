use super::super::{replay, ReportedProfilePrs, ReportedProfileRootSchemaV1};
use super::TestResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalAthlete, Gender, GradYear, ProfileDate, SchoolId, SourceIdentity, SourceNamespace,
};
use census_store::Store;
use serde_json::json;

fn journal_row() -> serde_json::Value {
    json!({
        "version": 1,
        "athlete_id": "14222592",
        "site": "AL",
        "profile_url": "https://al.milesplit.com/athletes/14222592-adelyn-spann",
        "acquired_at": "2026-10-26T00:00:00Z",
        "body_sha256": "abc123",
        "rows": [
            {
                "event": "High Jump",
                "mark": "5-0",
                "date": "May 2, 2026",
                "mode": "outdoor",
                "level": "all",
                "ordinal": 0,
                "byte_offset": 10,
                "byte_length": 20
            },
            {
                "event": "Long Jump",
                "mark": "14-8",
                "date": "Apr 18, 2026",
                "mode": "outdoor",
                "level": "all",
                "ordinal": 1,
                "byte_offset": 30,
                "byte_length": 20
            }
        ]
    })
}

fn athlete() -> TestResult<CanonicalAthlete> {
    Ok(CanonicalAthlete::new(
        &SchoolId::mint("sch", &["journal control"]),
        "Journal Control",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14222592"),
    )?)
}

#[test]
fn journal_rows_capture_replay_and_freeze() -> TestResult {
    use census_crawl::milesplit::PROFILE_PR_PHASE;
    let directory = tempfile::tempdir()?;
    let objects = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done(PROFILE_PR_PHASE, "9f:AL:14222592", &journal_row())?;
    let reported = ReportedProfilePrs::capture(
        &store.snapshot(),
        &[athlete()?],
        &ProfileDate::parse("2026-10-27")?,
        objects.path(),
        &mut AttachmentReplayGrantV1::new(),
        &crate::export::dataset::initial_objects::InitialObjects::fresh(),
    )?;
    check!(eq; reported.roots.len(), 1);
    let root = reported.roots.first().ok_or("journal root")?;
    check!(eq; root.schema, ReportedProfileRootSchemaV1::MilesplitProfilePrJournalV1);
    let serialized = serde_json::to_vec(&reported)?;
    let reopened: ReportedProfilePrs = serde_json::from_slice(&serialized)?;
    reported.require_initial_objects(
        objects.path(),
        &crate::export::dataset::initial_objects::InitialObjects::fresh(),
    )?;
    let bytes = crate::export::proof_objects::read(
        objects.path(),
        root.payload.digest.as_str(),
        root.payload.bytes,
    )?;
    let (subject, table) = replay::read(root, &bytes)?;
    check!(eq; subject, "14222592");
    check!(eq; table.profile_url, "https://al.milesplit.com/athletes/14222592-adelyn-spann");
    check!(eq; table.rows.len(), 2);
    check!(eq; table.rows.first().ok_or("first row")?.event(), "High Jump");
    check!(!reopened.is_empty());
    Ok(())
}

#[test]
fn journal_rows_before_cutoff_stay_out() -> TestResult {
    use census_crawl::milesplit::PROFILE_PR_PHASE;
    let directory = tempfile::tempdir()?;
    let objects = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done(PROFILE_PR_PHASE, "9f:AL:14222592", &journal_row())?;
    let reported = ReportedProfilePrs::capture(
        &store.snapshot(),
        &[athlete()?],
        &ProfileDate::parse("2026-10-25")?,
        objects.path(),
        &mut AttachmentReplayGrantV1::new(),
        &crate::export::dataset::initial_objects::InitialObjects::fresh(),
    )?;
    check!(reported.is_empty());
    Ok(())
}
