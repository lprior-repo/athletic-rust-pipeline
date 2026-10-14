use super::*;

fn captured(body: String, team: TeamRef) -> Result<RosterOutcome, Box<dyn std::error::Error>> {
    let verdict = parse_roster(&body, team.clone())?;
    Ok(RosterOutcome {
        verdict,
        capture: FetchOutcome {
            url: format!("{}/roster", team.url),
            response_url: None,
            method: "GET".to_string(),
            status: 200,
            content_digest: format!("{:x}", Sha256::digest(body.as_bytes())),
            bytes: body.len(),
            fetched_at: "2026-10-07T12:00:00Z".to_string(),
            from_cache: false,
            content_type: Some("text/html".to_string()),
            body: body.into_bytes(),
        },
    })
}

fn large_read() -> Result<RosterOutcome, Box<dyn std::error::Error>> {
    let mut team = team();
    team.name = "A".repeat(4096);
    let mut body = format!(
        r#"<link rel="canonical" href="{}/roster"><ul id="rosterDataset">"#,
        team.url
    );
    (0usize..2500).try_for_each(|index| {
        let id = index.checked_add(1).ok_or("fixture index overflow")?;
        body.push_str(&row(
            &id.to_string(),
            &format!("Runner, Person{index:04}"),
            "2027",
        ));
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    body.push_str("</ul>");
    captured(body, team)
}

async fn persist_prefix(store: &Store, run: &RosterRun<'_>, read: &RosterOutcome) -> TestResult {
    let roster = read.verdict.roster().ok_or("missing roster")?;
    let first = Window::next(roster, 0, run.observed_on)?.ok_or("missing first window")?;
    let prefix = first.athletes().len();
    let commit = Commit::new(store, &roster.team, run, read)?;
    let records = Records::of(&first, &run.site, run.school_year)?;
    let mut counts = Counts::new()?;
    counts.include(first.athletes(), records.teams())?;
    commit.window(Some(&records), &counts, 0, false).await?;
    check!(eq; athletes(store)?, (0..prefix).map(|index| format!("Person{index:04} Runner")).collect::<Vec<_>>());
    Ok(())
}

#[tokio::test]
async fn thousands_of_rows_resume_after_one_atomic_window_without_duplicate_facts() -> TestResult {
    let root = tempfile::tempdir()?;
    let run = run()?;
    let read = large_read()?;
    let roster = read.verdict.roster().ok_or("missing roster")?;
    {
        let store = Store::open(root.path())?;
        persist_prefix(&store, &run, &read).await?;
    }
    let store = Store::open(root.path())?;
    apply(&store, &roster.team, &run, &read).await?;
    apply(&store, &roster.team, &run, &read).await?;
    check!(eq; athletes(&store)?, (0..2500).map(|index| format!("Person{index:04} Runner")).collect::<Vec<_>>());
    check!(prior(&store, &roster.team, &run)?
        .ok_or("missing journal")?
        .is_terminal());
    let mut schools = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Schools, |school: CanonicalSchool| {
            schools.push(school.name);
            Ok(())
        })?;
    check!(eq; schools, vec![roster.team.name.clone()]);
    Ok(())
}

#[tokio::test]
async fn resource_refusal_retains_current_capture_and_prior_accepted_facts() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let run = run()?;
    apply(&store, &team(), &run, &capture(body(""))?).await?;
    let mut oversized = team();
    oversized.name = "A".repeat(4097);
    let read = captured(body("2027"), oversized.clone())?;
    check!(matches!(
        apply(&store, &oversized, &run, &read).await,
        Err(CrawlError::Resource { .. })
    ));
    let journal = prior(&store, &oversized, &run)?.ok_or("missing resource journal")?;
    check!(eq; journal.disposition, census_crawl::CollectionDisposition::Partial);
    check!(eq; journal.capture.ok_or("capture lost")?.content_digest, read.capture.content_digest);
    check!(eq; athletes(&store)?, vec!["Alice Runner".to_string()]);
    check!(eq; crate::census::scope::pending_rosters(&store, &[oversized], UsJurisdiction::Wisconsin, run.school_year, run.revision)?.len(), 1);
    Ok(())
}

#[tokio::test]
async fn disappeared_partial_team_restores_exact_retained_input_and_stays_owed() -> TestResult {
    use census_crawl::milesplit::TeamIndexRead;
    use census_crawl::CollectionDisposition as Disposition;
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let run = run()?;
    let jurisdiction = UsJurisdiction::Wisconsin;
    let complete = |teams| TeamIndexRead {
        teams,
        disposition: Disposition::Complete,
        errors: 0,
        unfinished: Vec::new(),
    };
    super::super::super::index::configure(&store, jurisdiction, complete(vec![team()]))?;
    apply(&store, &team(), &run, &capture(body(""))?).await?;
    let mut next = team();
    next.id = "52650".to_string();
    next.url = "https://wi.milesplit.com/teams/52650-example".to_string();
    let restored =
        super::super::super::index::configure(&store, jurisdiction, complete(vec![next]))?;
    check!(restored.iter().any(|restored| restored == &team()));
    let census_run = census_domain::model::CensusRun::new(run.school_year, run.revision.get())
        .ok_or("invalid run")?;
    let evidence = crate::census::inspect_rosters(&store, census_run, jurisdiction)?;
    check!(eq; (evidence.disposition, evidence.owed), (Disposition::Partial, 2));
    check!(eq; athletes(&store)?, vec!["Alice Runner".to_string()]);
    Ok(())
}
