use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};

fn seed_capture(cache: &Path, url: &str, body: &[u8]) {
    let key = Fetcher::key_for("GET", url, "");
    std::fs::create_dir_all(cache).expect("isolated cache");
    write_cache(
        &cache.join(format!("{key}.body")),
        &cache.join(format!("{key}.meta.json")),
        body,
        &CacheMeta {
            url: url.to_string(),
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: "2026-10-01T12:00:00Z".into(),
            ..CacheMeta::default()
        },
    )
    .expect("captured response");
}

#[tokio::test]
async fn same_name_association_owners_collect_distinct_coaches_and_ignore_legacy_name_receipts() {
    let dir = tempfile::tempdir().expect("isolated acquisition");
    let cache = dir.path().join("http");
    seed(
        &cache,
        1,
        &json!({"data": {"total": 1, "rows": [coach("Ada", "Lane")]}}).to_string(),
    );
    let second_url = coach_url(1).replace("EntityId=450", "EntityId=451");
    seed_capture(
        &cache,
        &second_url,
        &serde_json::to_vec(&json!({"data": {"total": 1, "rows": [coach("Beau", "Pine")]}}))
            .expect("second owner capture"),
    );
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = fetcher(&cache);
    let ctx = context(&fetcher, &store, None);
    let options = options();
    let first = school();
    let mut second = school();
    second.public_id = Some(451);
    let (_, natural_id) =
        crate::arbiter::map::map_org_school(&first, UsJurisdiction::NewHampshire, BASE, AT)
            .expect("unchanged canonical school contract");
    store
        .journal_done(
            "arbiter_orgs",
            natural_id.as_str(),
            &json!({"coach_rows": 0}),
        )
        .expect("historical name receipt");
    let mut acquisition = run(&ctx, &options);
    for row in [&first, &second] {
        acquisition
            .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
            .await
            .expect("source-owned acquisition");
    }
    assert_eq!(
        (
            acquisition.tally.schools,
            acquisition.tally.errors,
            acquisition.tally.skipped
        ),
        (2, 0, 0)
    );
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches).expect("retained coaches");
    assert_eq!(
        coaches
            .iter()
            .map(|row| row.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
    );
    let beau = coaches
        .iter()
        .find(|row| row.name == "Beau Pine")
        .expect("second source owner");
    assert_eq!(
        beau.evidence
            .first()
            .and_then(|evidence| evidence.source.url.as_deref()),
        Some(second_url.as_str())
    );
    assert_eq!(
        store.journal_keys(JOURNAL).expect("owned completions"),
        std::collections::HashSet::from(["NH:2132:450".into(), "NH:2132:451".into()])
    );
    assert_eq!(
        store
            .journal_keys("arbiter_orgs")
            .expect("historical receipt preserved"),
        std::collections::HashSet::from([natural_id.as_str().to_string()])
    );
    let mut replay = run(&ctx, &options);
    for row in [&first, &second] {
        replay
            .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
            .await
            .expect("owned replay");
    }
    assert_eq!((replay.tally.schools, replay.tally.skipped), (0, 2));
}

#[tokio::test]
async fn invalid_utf8_coach_capture_cannot_manufacture_a_replacement_character_person() {
    let dir = tempfile::tempdir().expect("isolated acquisition");
    let cache = dir.path().join("http");
    let body = b"{\"data\":{\"total\":1,\"rows\":[{\"firstName\":\"\xff\",\"lastName\":\"Lane\",\"coachPositionName\":\"Head Coach\",\"sportName\":\"Cross Country, Girls\",\"levelName\":\"Varsity\"}]}}";
    seed_capture(&cache, &coach_url(1), body);
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = fetcher(&cache);
    let ctx = context(&fetcher, &store, None);
    let options = options();
    let mut acquisition = run(&ctx, &options);
    acquisition
        .process_school(UsJurisdiction::NewHampshire, "2132", &school(), BASE)
        .await
        .expect("retained valid directory facts");
    assert_eq!(
        (acquisition.tally.schools, acquisition.tally.errors),
        (1, 1)
    );
    assert_retained_facts(&store, &BTreeSet::from(["Casey Reed".to_string()]));
    assert!(acquisition
        .tally
        .notes
        .iter()
        .any(|note| note.contains("coach response is not UTF-8")));
}

#[tokio::test]
async fn oversized_coach_page_is_a_bounded_failure_without_owned_completion() {
    let dir = tempfile::tempdir().expect("isolated acquisition");
    let cache = dir.path().join("http");
    let rows = vec![coach("Ada", "Lane"); 201];
    seed(
        &cache,
        1,
        &json!({"data": {"total": 201, "rows": rows}}).to_string(),
    );
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = fetcher(&cache);
    let ctx = context(&fetcher, &store, None);
    let options = options();
    let mut acquisition = run(&ctx, &options);
    acquisition
        .process_school(UsJurisdiction::NewHampshire, "2132", &school(), BASE)
        .await
        .expect("bounded page outcome");
    assert_eq!(
        (acquisition.tally.schools, acquisition.tally.errors),
        (1, 1)
    );
    assert_retained_facts(&store, &BTreeSet::from(["Casey Reed".to_string()]));
    assert!(acquisition
        .tally
        .notes
        .iter()
        .any(|note| note.contains("page exceeds requested page size")));
}
