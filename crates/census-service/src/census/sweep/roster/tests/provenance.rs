use super::*;
use census_domain::model::SourceObservation;

#[tokio::test]
async fn positive_rows_retain_capture_coordinates_across_replay_and_reopen() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut run = run()?;
    run.observed_on = "2031-10-12";
    let read = capture(format!("é🏃{}", body("2027")))?;
    apply(&store, &team(), &run, &read).await?;
    let before = store.snapshot().tables_digest(&Table::ALL)?;
    apply(&store, &team(), &run, &read).await?;
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, before);
    let rows: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let observations: Vec<SourceObservation> = store.scan(Table::SourceObservations)?;
    let roster = read.verdict.roster().ok_or("roster")?;
    check!(eq; rows.len(), 2);
    for member in &roster.athletes {
        let row = rows
            .iter()
            .find(|row| {
                row.identities().any(|identity| {
                    identity.namespace == census_domain::model::SourceNamespace::MilesplitAthlete
                        && identity.id.as_str() == member.athlete().athlete_id
                })
            })
            .ok_or("athlete")?;
        let fact = row
            .evidence
            .iter()
            .find(|fact| fact.source.url.as_deref() == Some(read.capture.url.as_str()))
            .ok_or("roster evidence")?;
        check!(eq; fact.observed_on, read.capture.fetched_at);
        let note: serde_json::Value =
            serde_json::from_str(fact.note.as_deref().ok_or("capture binding")?)?;
        check!(eq; note["row"], serde_json::to_value(member.locator())?);
        check!(eq; note["capture_sha256"], read.capture.content_digest);
        check!(eq; note["capture_bytes"], read.capture.bytes);
        check!(eq; note["provider_athlete_id"], member.athlete().athlete_id);
        check!(eq; note["provider_team_id"], team().id);
        let observation = observations
            .iter()
            .find_map(|row| match row {
                SourceObservation::Athlete(row)
                    if row.source_athlete_id == member.athlete().athlete_id =>
                {
                    Some(row)
                }
                _ => None,
            })
            .ok_or("source observation")?;
        check!(eq; observation.source_row_key, note["source_row_key"].as_str().ok_or("row key")?);
        check!(eq; observation.profile_url.as_deref(), Some(member.athlete().profile_url.as_str()));
    }
    drop(store);
    let reopened = Store::open(root.path())?;
    check!(eq; reopened.scan::<CanonicalAthlete>(Table::Athletes)?, rows);
    check!(eq; reopened.scan::<SourceObservation>(Table::SourceObservations)?, observations);
    Ok(())
}

#[tokio::test]
async fn malformed_capture_bindings_never_emit_positive_rows() -> TestResult {
    for mutation in 0..8 {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let mut read = capture(body("2027"))?;
        match mutation {
            0 => read.capture.content_digest = "0".repeat(64),
            1 => read.capture.bytes = 1,
            2 => read.capture.url = "https://wi.milesplit.com/teams/1-other/roster".into(),
            3 => read.capture.fetched_at = "not-a-time".into(),
            4 => read.verdict = capture(format!("prefix{}", body("2027")))?.verdict,
            5 => read.verdict = capture(body("2028"))?.verdict,
            6 => {
                read.capture.response_url =
                    Some("https://wi.milesplit.com/teams/1-other/roster".into())
            }
            _ => read.capture.method = "POST".into(),
        }
        check!(apply(&store, &team(), &run()?, &read).await.is_err());
        check!(store.scan::<CanonicalAthlete>(Table::Athletes)?.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn changed_capture_retains_both_physical_positive_bindings() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let first = capture(body("2027"))?;
    let second = capture(format!("é{}", body("2027")))?;
    apply(&store, &team(), &run()?, &first).await?;
    apply(&store, &team(), &run()?, &second).await?;
    check!(eq; store.walk_table(Table::Athletes)?.rows, 4);
    let rows: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    check!(eq; rows.len(), 2);
    for row in rows {
        for digest in [
            &first.capture.content_digest,
            &second.capture.content_digest,
        ] {
            check!(row
                .evidence
                .iter()
                .any(|fact| fact.note.as_ref().is_some_and(|note| note.contains(digest))));
        }
    }
    Ok(())
}

#[tokio::test]
async fn positive_rows_keep_global_coordinates_after_rejection_and_window_boundary() -> TestResult {
    let mut html = format!(
        "é🏃<link rel=\"canonical\" href=\"{}/roster\"><ul id=\"rosterDataset\">",
        team().url
    );
    let mut expected = Vec::new();
    for ordinal in 0u32..67 {
        let id = ordinal.checked_add(1).ok_or("id")?;
        let text = row(
            &id.to_string(),
            &format!("Runner, Person{ordinal}"),
            if ordinal == 31 { "invalid" } else { "2027" },
        );
        let opening = "<li class=\"athlete-row data-row\">";
        let inner = text
            .strip_prefix(opening)
            .and_then(|text| text.strip_suffix("</li>"))
            .ok_or("row")?;
        if ordinal != 31 {
            expected.push((
                id.to_string(),
                SourceRowLocator {
                    ordinal,
                    byte_offset: html.len().checked_add(opening.len()).ok_or("offset")?,
                    byte_length: inner.len(),
                },
            ));
        }
        html.push_str(&text);
    }
    html.push_str("</ul>");
    let read = capture(html)?;
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    apply(&store, &team(), &run()?, &read).await?;
    let rows: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    check!(eq; rows.len(), 66);
    for (id, locator) in expected {
        let athlete = rows
            .iter()
            .find(|row| row.identities().any(|owner| owner.id.as_str() == id))
            .ok_or("athlete")?;
        let note: serde_json::Value = serde_json::from_str(
            athlete
                .evidence
                .first()
                .and_then(|fact| fact.note.as_deref())
                .ok_or("binding")?,
        )?;
        check!(eq; note["row"], serde_json::to_value(locator)?);
    }
    Ok(())
}

#[tokio::test]
async fn same_owner_response_paths_preserve_original_request_binding() -> TestResult {
    for response_url in [
        "https://www.milesplit.com/teams/52649/roster",
        "https://wi.milesplit.com/teams/52649-renamed/roster",
    ] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let mut read = capture(body("2027"))?;
        read.capture.response_url = Some(response_url.to_string());
        apply(&store, &team(), &run()?, &read).await?;
        let rows: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
        check!(eq; rows.len(), 2);
        for athlete in rows {
            let evidence = athlete.evidence.first().ok_or("roster evidence")?;
            check!(eq; evidence.source.url.as_deref(), Some(read.capture.url.as_str()));
            let note: serde_json::Value =
                serde_json::from_str(evidence.note.as_deref().ok_or("binding")?)?;
            check!(eq; note["response_url"], response_url);
            check!(eq; note["capture_url"], read.capture.url);
        }
    }
    Ok(())
}

#[tokio::test]
async fn foreign_response_owners_never_emit_positive_rows() -> TestResult {
    for response_url in [
        "https://wi.milesplit.com/teams/26848/roster",
        "https://oh.milesplit.com/teams/52649/roster",
        "https://wi.milesplit.com/teams/052649/roster",
        "https://wi.milesplit.com/teams/52649",
        "https://milesplit.com.attacker.test/teams/52649/roster",
    ] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let mut read = capture(body("2027"))?;
        read.capture.response_url = Some(response_url.to_string());
        check!(apply(&store, &team(), &run()?, &read).await.is_err());
        check!(store.scan::<CanonicalAthlete>(Table::Athletes)?.is_empty());
        check!(store
            .scan::<SourceObservation>(Table::SourceObservations)?
            .is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn oversized_same_owner_response_metadata_is_refused() -> TestResult {
    for separator in ['?', '#'] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let mut read = capture(body("2027"))?;
        read.capture.response_url = Some(format!(
            "https://wi.milesplit.com/teams/52649/roster{separator}{}",
            "a".repeat(4096)
        ));
        check!(apply(&store, &team(), &run()?, &read).await.is_err());
        check!(store.scan::<CanonicalAthlete>(Table::Athletes)?.is_empty());
        check!(store
            .scan::<SourceObservation>(Table::SourceObservations)?
            .is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn complete_verdict_cannot_hide_rejected_captured_rows() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut read = capture(body("invalid"))?;
    let roster = read.verdict.roster().ok_or("partial roster")?.clone();
    read.verdict = milesplit::RosterVerdict::Complete { roster };
    check!(apply(&store, &team(), &run()?, &read).await.is_err());
    check!(store.scan::<CanonicalAthlete>(Table::Athletes)?.is_empty());
    check!(store
        .scan::<SourceObservation>(Table::SourceObservations)?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn legacy_rows_and_receipt_survive_bound_update_reopen_and_replay() -> TestResult {
    let root = tempfile::tempdir()?;
    let run = run()?;
    let read = capture(body("2027"))?;
    let team = team();
    let roster = read.verdict.roster().ok_or("roster")?;
    let window = Window::next(roster, 0, run.observed_on)?.ok_or("window")?;
    let records = Records::of(&window, &run.site, run.school_year)?;
    let mut counts = Counts::new()?;
    counts.include(records.members(), records.teams())?;
    let store = Store::open(root.path())?;
    let facts = records.stage(&store, &mut store.write_batch())?;
    let digest = super::super::journal::roster_digest(&facts, &team, &run, &read, 0)?;
    let operation = format!(
        "{}:{digest}",
        super::super::journal::roster_operation(
            run.site.jurisdiction().code(),
            run.school_year,
            run.revision,
            &team
        )
    );
    Commit::new(&store, &team, &run, &read)?
        .window(Some(&records), &counts, 0, true)
        .await?;
    let mut old = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Athletes, |row: CanonicalAthlete| {
            old.push(row);
            Ok(())
        })?;
    drop(store);
    let store = Store::open(root.path())?;
    apply(&store, &team, &run, &read).await?;
    let mut updated = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Athletes, |row: CanonicalAthlete| {
            updated.push(row);
            Ok(())
        })?;
    check!(eq; updated.len(), 4);
    check!(old.iter().all(|row| updated.contains(row)));
    drop(store);
    let store = Store::open(root.path())?;
    let before = store.snapshot().tables_digest(&Table::ALL)?;
    apply(&store, &team, &run, &read).await?;
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, before);
    check!(!store
        .write_batch()
        .commit_once(&operation, &digest)?
        .written());
    Ok(())
}

#[tokio::test]
async fn bounded_response_metadata_fits_reserved_attachment_budget() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut read = capture(body("2027"))?;
    let prefix = "https://wi.milesplit.com/teams/52649/roster?";
    read.capture.response_url = Some(format!("{prefix}{}", "a".repeat(4096 - prefix.len())));
    apply(&store, &team(), &run()?, &read).await?;
    for athlete in store.scan::<CanonicalAthlete>(Table::Athletes)? {
        let fact = athlete.evidence.first().ok_or("evidence")?;
        let note = fact.note.as_deref().ok_or("note")?;
        let value: serde_json::Value = serde_json::from_str(note)?;
        let key = value["source_row_key"].as_str().ok_or("key")?;
        let url = fact.source.url.as_deref().ok_or("url")?;
        let bytes = note.len() + key.len() + url.len();
        check!(bytes <= super::super::provenance::MAX_ATTACHMENT_BYTES);
        check!(bytes * 64 <= super::super::provenance::MAX_WINDOW_ATTACHMENT_BYTES);
    }
    check!(super::super::provenance::MAX_WINDOW_ATTACHMENT_BYTES < 8 * 1024 * 1024);
    Ok(())
}
