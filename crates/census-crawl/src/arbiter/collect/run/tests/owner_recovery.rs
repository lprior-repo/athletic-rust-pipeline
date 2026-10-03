use super::*;

#[test]
fn same_name_incomplete_public_owners_refresh_independently_and_preserve_v1_history() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let host = format!("http://{}", listener.local_addr()?);
    let first_url = local_url(&host, 450, 1);
    let second_url = local_url(&host, 451, 1);
    let first_bad = serde_json::to_vec(&json!({"data": {"total": 2, "rows": [coach("Ada", "Lane")]}}))?;
    let second_bad = serde_json::to_vec(&json!({"data": {"total": 2, "rows": [coach("Beau", "Pine")]}}))?;
    let first_good = serde_json::to_vec(&json!({"data": {"total": 1, "rows": [coach("Ada", "Lane")]}}))?;
    let second_good = serde_json::to_vec(&json!({"data": {"total": 1, "rows": [coach("Beau", "Pine")]}}))?;
    let replies = vec![
        robots(),
        reply(&host, &first_url, 200, &first_bad)?,
        reply(&host, &second_url, 200, &second_bad)?,
        reply(&host, &first_url, 200, &first_good)?,
        reply(&host, &second_url, 200, &second_good)?,
    ];
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = local_fetcher(&cache)?;
    let ctx = context(&fetcher, &store, None)?;
    let options = options();
    let first = school();
    let mut second = school();
    second.public_id = Some(451);
    let natural_id = crate::arbiter::map::map_org_school(&first, UsJurisdiction::NewHampshire, BASE, AT)
        .ok_or("first canonical id")?.1;
    check!(eq;
        natural_id,
        crate::arbiter::map::map_org_school(&second, UsJurisdiction::NewHampshire, BASE, AT)
            .ok_or("second canonical id")?.1
    );
    let historic_phase = format!("arbiter_coaches_incomplete_v1:{}", natural_id.as_str());
    let historic = json!({"version": 1, "school_id": natural_id, "kind": "partial", "failure": "historical unowned failure", "recovery": {"responses": []}});
    store.journal_done(&historic_phase, "pending", &historic)?;
    let client = async {
        for row in [&first, &second] {
            let mut failed = run(&ctx, &options)?;
            failed.process_school_at(UsJurisdiction::NewHampshire, "2132", row, BASE, &host).await?;
            check!(eq; (failed.tally.errors, failed.done.len()), (1, 0));
        }
        let first_pending = pending(&store, &first)?;
        let second_pending = pending(&store, &second)?;
        check!(eq; first_pending["owner_key"], json!("NH:2132:450"));
        check!(eq; second_pending["owner_key"], json!("NH:2132:451"));
        check!(eq; first_pending["recovery"]["responses"][0]["url"], json!(first_url));
        check!(eq; second_pending["recovery"]["responses"][0]["url"], json!(second_url));
        let first_meta = capture_meta(&cache, &first_url)?;
        let second_meta = capture_meta(&cache, &second_url)?;
        let partial: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
        check!(eq; partial.len(), 3);
        check!(eq;
            partial.iter().map(|coach| coach.name.as_str()).collect::<BTreeSet<_>>(),
            BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
        );
        check!(eq; store.journal_keys(JOURNAL)?, std::collections::HashSet::new());
        let mut recovered = run(&ctx, &options)?;
        recovered.process_school_at(UsJurisdiction::NewHampshire, "2132", &first, BASE, &host).await?;
        check!(eq; (recovered.tally.errors, recovered.tally.schools), (0, 1));
        check!(eq; store.journal_keys(JOURNAL)?, std::collections::HashSet::from(["NH:2132:450".to_string()]));
        check!(eq; pending(&store, &second)?, second_pending);
        recovered.process_school_at(UsJurisdiction::NewHampshire, "2132", &second, BASE, &host).await?;
        check!(eq; (recovered.tally.errors, recovered.tally.schools), (0, 2));
        check!(eq;
            store.journal_keys(JOURNAL)?,
            std::collections::HashSet::from(["NH:2132:450".to_string(), "NH:2132:451".to_string()])
        );
        let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
        check!(eq; coaches.len(), 3);
        check!(eq;
            coaches.iter().map(|coach| coach.name.as_str()).collect::<BTreeSet<_>>(),
            BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
        );
        assert_archived_capture(&cache, &first_bad, &first_meta)?;
        assert_archived_capture(&cache, &second_bad, &second_meta)?;
        check!(eq; store.journal_payloads(&historic_phase)?, vec![historic]);
        let before = fetcher.stats().await.requests;
        let mut replay = run(&ctx, &options)?;
        for row in [&first, &second] {
            replay.process_school_at(UsJurisdiction::NewHampshire, "2132", row, BASE, &host).await?;
        }
        check!(eq; (replay.tally.schools, replay.tally.skipped), (0, 2));
        check!(eq; fetcher.stats().await.requests, before);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        let (served, acquired) = tokio::join!(serve_responses(listener, replies), client);
        served?;
        acquired
    }).await?
    })
}

#[test]
fn recovery_payload_for_another_public_owner_is_rejected_even_when_school_id_aliases() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let school_id =
        crate::arbiter::map::map_org_school(&school(), UsJurisdiction::NewHampshire, BASE, AT)
            .ok_or("canonical id")?
            .1;
    let phase = super::super::recovery::phase("NH:2132:450");
    store.journal_done(
        &phase,
        "pending",
        &json!({
            "version": 2, "owner_key": "NH:2132:451", "school_id": school_id,
            "kind": "partial", "failure": "another public owner's failure",
            "recovery": {"responses": []}
        }),
    )?;
    match super::super::recovery::Recovery::load(&store, &school_id, "NH:2132:450") {
        Err(crate::CrawlError::Schema { url, .. }) => {
            check!(eq; url, phase);
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("another owner's marker must not be adopted".into()),
    }
    Ok(())
}
