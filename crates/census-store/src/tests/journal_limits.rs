use super::*;

#[test]
fn journal_roundtrips_resume_keys() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:52649",
            &serde_json::json!({"athletes": 59}),
        )
        .unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:26848",
            &serde_json::json!({"athletes": 0}),
        )
        .unwrap();
    let keys = store.journal_keys("milesplit_rosters").unwrap();
    assert!(keys.contains("wi:52649"));
    assert_eq!(keys.len(), 2);
    assert_eq!(
        store.journal_payloads("milesplit_rosters").unwrap().len(),
        2
    );
    store
        .journal_done("other_phase", "wi:1", &serde_json::json!({}))
        .unwrap();
    assert_eq!(store.journal_keys("milesplit_rosters").unwrap().len(), 2);
}

#[test]
fn a_journal_key_past_its_ceiling_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let key = "k".repeat(MAX_JOURNAL_KEY_BYTES + 1);

    match store.journal_done("ihsa_schools", &key, &serde_json::json!({"rows": 1})) {
        Err(StoreError::JournalTooLarge {
            what,
            phase,
            bytes,
            max,
            ..
        }) => {
            assert_eq!(what, "key");
            assert_eq!(phase, "ihsa_schools");
            assert_eq!(max, MAX_JOURNAL_KEY_BYTES);
            assert!(bytes > max, "the refusal names what it measured: {bytes}");
        }
        other => panic!("expected the journal key ceiling to refuse the entry, got {other:?}"),
    }
    assert!(store.journal_keys("ihsa_schools").unwrap().is_empty());
    assert!(store.journal_payloads("ihsa_schools").unwrap().is_empty());
}

#[test]
fn a_journal_value_past_its_ceiling_leaves_the_phase_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .journal_done("ihsa_schools", "kept", &serde_json::json!({"rows": 1}))
        .unwrap();

    let payload = "v".repeat(MAX_JOURNAL_VALUE_BYTES + 1);
    match store.journal_done("ihsa_schools", "refused", &serde_json::json!(payload)) {
        Err(StoreError::JournalTooLarge {
            what,
            key,
            bytes,
            max,
            ..
        }) => {
            assert_eq!(what, "value");
            assert_eq!(key, "refused");
            assert_eq!(max, MAX_JOURNAL_VALUE_BYTES);
            assert!(bytes > max, "the refusal names what it measured: {bytes}");
        }
        other => panic!("expected the journal value ceiling to refuse the entry, got {other:?}"),
    }
    let keys = store.journal_keys("ihsa_schools").unwrap();
    assert!(keys.contains("kept"));
    assert!(!keys.contains("refused"));
    assert_eq!(store.journal_payloads("ihsa_schools").unwrap().len(), 1);
}
