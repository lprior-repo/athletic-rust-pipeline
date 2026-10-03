use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};

fn seed_capture(cache: &Path, url: &str, body: &[u8]) -> TestResult {
    let key = Fetcher::key_for("GET", url, "");
    std::fs::create_dir_all(cache)?;
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
    )?;
    Ok(())
}

#[test]
fn same_name_association_owners_collect_distinct_coaches_and_ignore_legacy_name_receipts(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            seed(
                &cache,
                1,
                &json!({"data": {"total": 1, "rows": [coach("Ada", "Lane")]}}).to_string(),
            )?;
            let second_url = coach_url(1).replace("EntityId=450", "EntityId=451");
            seed_capture(
                &cache,
                &second_url,
                &serde_json::to_vec(
                    &json!({"data": {"total": 1, "rows": [coach("Beau", "Pine")]}}),
                )?,
            )?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = fetcher(&cache)?;
            let ctx = context(&fetcher, &store, None)?;
            let options = options();
            let first = school();
            let mut second = school();
            second.public_id = Some(451);
            let (_, natural_id) =
                crate::arbiter::map::map_org_school(&first, UsJurisdiction::NewHampshire, BASE, AT)
                    .ok_or("unchanged canonical school contract")?;
            store.journal_done(
                "arbiter_orgs",
                natural_id.as_str(),
                &json!({"coach_rows": 0}),
            )?;
            let mut acquisition = run(&ctx, &options)?;
            for row in [&first, &second] {
                acquisition
                    .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
                    .await?;
            }
            check!(eq;
                (acquisition.tally.schools, acquisition.tally.errors, acquisition.tally.skipped),
                (2, 0, 0)
            );
            let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
            check!(eq;
                coaches.iter().map(|row| row.name.as_str()).collect::<BTreeSet<_>>(),
                BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
            );
            let beau = coaches
                .iter()
                .find(|row| row.name == "Beau Pine")
                .ok_or("second source owner")?;
            check!(eq;
                beau.evidence.first().and_then(|evidence| evidence.source.url.as_deref()),
                Some(second_url.as_str())
            );
            check!(eq;
                store.journal_keys(JOURNAL)?,
                std::collections::HashSet::from(["NH:2132:450".into(), "NH:2132:451".into()])
            );
            check!(eq;
                store.journal_keys("arbiter_orgs")?,
                std::collections::HashSet::from([natural_id.as_str().to_string()])
            );
            let mut replay = run(&ctx, &options)?;
            for row in [&first, &second] {
                replay
                    .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
                    .await?;
            }
            check!(eq; (replay.tally.schools, replay.tally.skipped), (0, 2));
            Ok(())
        })
}

#[test]
fn invalid_utf8_coach_capture_cannot_manufacture_a_replacement_character_person() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let body = b"{\"data\":{\"total\":1,\"rows\":[{\"firstName\":\"\xff\",\"lastName\":\"Lane\",\"coachPositionName\":\"Head Coach\",\"sportName\":\"Cross Country, Girls\",\"levelName\":\"Varsity\"}]}}";
    seed_capture(&cache, &coach_url(1), body)?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = fetcher(&cache)?;
    let ctx = context(&fetcher, &store, None)?;
    let options = options();
    let mut acquisition = run(&ctx, &options)?;
    acquisition.process_school(UsJurisdiction::NewHampshire, "2132", &school(), BASE).await?;
    check!(eq; (acquisition.tally.schools, acquisition.tally.errors), (1, 1));
    assert_retained_facts(&store, &BTreeSet::from(["Casey Reed".to_string()]))?;
    check!(acquisition.tally.notes.iter().any(|note| note.contains("coach response is not UTF-8")));
    Ok(())
    })
}

#[test]
fn oversized_coach_page_is_a_bounded_failure_without_owned_completion() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            let rows = vec![coach("Ada", "Lane"); 201];
            seed(
                &cache,
                1,
                &json!({"data": {"total": 201, "rows": rows}}).to_string(),
            )?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = fetcher(&cache)?;
            let ctx = context(&fetcher, &store, None)?;
            let options = options();
            let mut acquisition = run(&ctx, &options)?;
            acquisition
                .process_school(UsJurisdiction::NewHampshire, "2132", &school(), BASE)
                .await?;
            check!(eq; (acquisition.tally.schools, acquisition.tally.errors), (1, 1));
            assert_retained_facts(&store, &BTreeSet::from(["Casey Reed".to_string()]))?;
            check!(acquisition
                .tally
                .notes
                .iter()
                .any(|note| note.contains("page exceeds requested page size")));
            Ok(())
        })
}
