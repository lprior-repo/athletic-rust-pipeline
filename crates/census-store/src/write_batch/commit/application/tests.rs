use super::*;
use chrono::SecondsFormat;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn sampled_commit_instant_binds_journal_and_receipt_across_utc_midnight() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let _appends = store.lock_appends();
    for (key, instant, expected_day) in [
        ("before", "2026-05-15T23:59:59Z", "2026-05-15"),
        ("after", "2026-05-16T00:00:00Z", "2026-05-16"),
    ] {
        let sampled = DateTime::parse_from_rfc3339(instant)?.with_timezone(&Utc);
        let timestamp = sampled.to_rfc3339_opts(SecondsFormat::Secs, true);
        let payload = json!({"committed_at": timestamp});
        let envelope = json!({"key": key, "at": timestamp, "payload": payload});
        let mut prepared = store.write_batch();
        prepared.journal.push((
            Store::journal_key("sampled_time", key),
            serde_json::to_vec(&envelope)?,
        ));
        let application = prepared
            .commit_staged_at(
                Some((key, "sampled-digest")),
                store.db.batch(),
                Some(sampled),
            )?
            .ok_or("missing sampled receipt")?;
        assert!(application.written());
        assert_eq!(application.receipt().at, expected_day);
        assert_eq!(application.receipt().appended, 0);
        assert!(!application.receipt().credited);
        assert_eq!(store.receipt(key)?.as_ref(), Some(application.receipt()));
        let snapshot = store.snapshot_locked();
        let held = snapshot
            .journal_entry_raw_bounded("sampled_time", key, crate::JournalReadBudget::new(4096)?)?
            .ok_or("missing sampled envelope")?;
        assert_eq!(held.at(), instant);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(held.payload().get())?,
            payload
        );
    }
    Ok(())
}
