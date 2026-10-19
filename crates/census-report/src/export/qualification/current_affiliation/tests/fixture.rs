use super::*;
use census_crawl::milesplit::{parse_captured_roster, roster_entities, Site, TeamRef};
use census_crawl::net::FetchOutcome;
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use serde_json::json;

pub(in super::super) struct Fixture {
    pub _directory: tempfile::TempDir,
    pub store: Store,
    pub writer: census_store::PublicIdentityWriterV1,
    pub athlete: AthleteId,
    pub school: SchoolId,
    pub profile: QueryProfileV1,
    pub body_path: std::path::PathBuf,
    pub metadata_path: std::path::PathBuf,
}

impl Fixture {
    pub fn new(captured: &str, cutoff: &str, accept_identity: bool) -> TestResult<Self> {
        Self::with_heading(captured, cutoff, accept_identity, "Current Roster")
    }

    pub fn with_heading(
        captured: &str,
        cutoff: &str,
        accept_identity: bool,
        heading: &str,
    ) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        Store::create_public_identity_qualification_v3(directory.path())?;
        let (store, writer) =
            Store::open_public_identity_qualification_v3(directory.path())?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let profile = profile(cutoff)?;
        store.register_profile(&profile)?;
        let (athlete, school, body_path, metadata_path) = append_roster_heading(
            &store,
            "52649",
            "Old High",
            captured,
            Sport::OutdoorTrack,
            heading,
        )?;
        if accept_identity {
            accept(&store, &writer, &athlete)?;
        }
        Ok(Self {
            _directory: directory,
            store,
            writer,
            athlete,
            school,
            profile,
            body_path,
            metadata_path,
        })
    }

    pub fn dataset(&self, snapshot: &census_store::StoreSnapshot<'_>) -> TestResult<ExportDataset> {
        let mut dataset =
            ExportDataset::load_public_identity_snapshot_fixture(&self.store, snapshot)?;
        dataset.bind_selection_for_test(self.profile.id()?, self.profile.input().as_of.clone())?;
        dataset.bind_public_identity_fixture_from_snapshot(snapshot)?;
        Ok(dataset)
    }
}

pub(in super::super) fn accept(
    store: &Store,
    writer: &census_store::PublicIdentityWriterV1,
    athlete: &AthleteId,
) -> TestResult {
    writer.establish_source_bound(
        store,
        &format!("source-bound-roster-fixture-{athlete}"),
        &athlete.cast(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    Ok(())
}

pub(in super::super) fn append_roster(
    store: &Store,
    team_id: &str,
    name: &str,
    at: &str,
) -> TestResult<(AthleteId, SchoolId, std::path::PathBuf, std::path::PathBuf)> {
    append_roster_sport(store, team_id, name, at, Sport::OutdoorTrack)
}

pub(in super::super) fn append_roster_sport(
    store: &Store,
    team_id: &str,
    name: &str,
    at: &str,
    sport: Sport,
) -> TestResult<(AthleteId, SchoolId, std::path::PathBuf, std::path::PathBuf)> {
    append_roster_heading(store, team_id, name, at, sport, "Current Roster")
}

pub(in super::super) fn append_roster_heading(
    store: &Store,
    team_id: &str,
    name: &str,
    at: &str,
    sport: Sport,
    heading: &str,
) -> TestResult<(AthleteId, SchoolId, std::path::PathBuf, std::path::PathBuf)> {
    let team = TeamRef {
        id: team_id.into(),
        slug: "example".into(),
        url: format!("https://wi.milesplit.com/teams/{team_id}-example"),
        name: name.into(),
        city_state: "Madison, WI".into(),
    };
    let cells = [Sport::IndoorTrack, Sport::OutdoorTrack, Sport::CrossCountry]
        .into_iter()
        .zip([1, 2, 3])
        .map(|(candidate, season)| {
            let active = if candidate == sport { "yes" } else { "no" };
            format!(r#"<div class="data-point" data-season-id="{season}"><svg class="icon icon-{active}"></svg></div>"#)
        })
        .collect::<String>();
    let body = format!(r#"<link rel="canonical" href="{}/roster"><section><h4>{heading}</h4><ul id="rosterDataset"><li class="athlete-row data-row"><a href="https://wi.milesplit.com/athletes/1-runner">Runner, Alice</a><div class="column-gender">f</div><div class="column-grad-year">2027</div>{cells}</li></ul></section>"#, team.url).into_bytes();
    let digest = archive::hash(&body);
    let capture = FetchOutcome {
        url: format!("{}/roster", team.url),
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: digest.clone(),
        bytes: body.len(),
        fetched_at: at.into(),
        from_cache: false,
        content_type: Some("text/html".into()),
        body,
    };
    let verdict =
        parse_captured_roster(std::str::from_utf8(&capture.body)?, team.clone(), &capture)?;
    let roster = verdict.roster().ok_or("roster")?;
    let member = roster.athletes.first().ok_or("member")?;
    let (school, mut athletes, _) = roster_entities(
        &team,
        &roster.athletes,
        SchoolYear::DEFAULT,
        at,
        &Site::for_jurisdiction(UsJurisdiction::Wisconsin),
    )?;
    let athlete = athletes.first_mut().ok_or("athlete")?;
    let row = member.locator();
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("row overflow")?;
    let span = capture.body.get(row.byte_offset..end).ok_or("row span")?;
    let key = format!(
        "{}#capture={digest}&row={}&offset={}&length={}",
        capture.url, row.ordinal, row.byte_offset, row.byte_length
    );
    let note = json!({
        "role": "roster_positive_row_metadata_v1", "source_row_key": key,
        "provider_team_id": team_id, "provider_athlete_id": "1", "capture_url": capture.url,
        "response_url": null, "capture_sha256": digest, "capture_bytes": capture.bytes,
        "fetched_at": at, "row": row, "row_sha256": archive::hash(span),
    });
    let evidence = athlete.evidence.first_mut().ok_or("source evidence")?;
    evidence.source.url = Some(capture.url.clone());
    evidence.note = Some(serde_json::to_string(&note)?);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, athlete)?;
    let root = store.http_cache_dir().join("archive");
    std::fs::create_dir_all(root.join("bodies"))?;
    std::fs::create_dir_all(root.join("captures").join(&digest))?;
    let body_path = root.join("bodies").join(format!("{digest}.body"));
    std::fs::write(&body_path, &capture.body)?;
    let metadata = serde_json::to_vec(&json!({
        "url": capture.url, "response_url": null, "method": "GET", "status": 200,
        "content_digest": digest, "bytes": capture.bytes, "fetched_at": at,
        "content_type": "text/html", "etag": "fixture", "redirects": [],
    }))?;
    let metadata_path = root
        .join("captures")
        .join(&digest)
        .join(format!("{}.meta.json", archive::hash(&metadata)));
    std::fs::write(&metadata_path, metadata)?;
    Ok((athlete.id.clone(), school.id, body_path, metadata_path))
}

fn profile(as_of: &str) -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: ProfileDate::parse(as_of)?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-31")?,
        )?,
        completion_contract: reference(Role::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: reference(Role::Cohort)?,
            event: reference(Role::Event)?,
            identity: reference(Role::Identity)?,
            affiliation: reference(Role::Affiliation)?,
            contact: reference(Role::Contact)?,
            participation: reference(Role::Participation)?,
            publication: reference(Role::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}
