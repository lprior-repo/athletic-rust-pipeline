use super::{CanonicalAthlete, CanonicalSchool, RosterCase, Store, TestResult};
use census_crawl::milesplit::{Site, TeamRef};
use census_domain::model::{
    Evidence, PublishedGraduation, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_store::Table;

pub(super) fn append(
    store: &Store,
    original: &CanonicalAthlete,
    school: &CanonicalSchool,
    case: RosterCase,
) -> TestResult<CanonicalAthlete> {
    let owner = school
        .source_identities
        .first()
        .ok_or("roster school owner")?;
    let person = original.source.as_ref().ok_or("original source")?;
    let team = TeamRef {
        id: owner.id.clone(),
        slug: "synthetic-public-consumer".into(),
        url: format!(
            "https://wi.milesplit.com/teams/{}-synthetic-public-consumer",
            owner.id
        ),
        name: school.name.clone(),
        city_state: "WI".into(),
    };
    let heading = if matches!(case, RosterCase::Undated) {
        "Roster"
    } else {
        "Current Roster"
    };
    let captured = if matches!(case, RosterCase::OutsideCaptureDay) {
        "2026-09-30T12:00:00Z"
    } else {
        "2026-10-01T12:00:00Z"
    };
    let body = format!(r#"<link rel="canonical" href="{}/roster"><section><h4>{heading}</h4><ul id="rosterDataset"><li class="athlete-row data-row"><a href="https://wi.milesplit.com/athletes/{}-synthetic-public-consumer">{}</a><div class="column-gender">f</div><div class="column-grad-year">2027</div><div class="data-point" data-season-id="1"><svg class="icon icon-no"></svg></div><div class="data-point" data-season-id="2"><svg class="icon icon-yes"></svg></div><div class="data-point" data-season-id="3"><svg class="icon icon-no"></svg></div></li></ul></section>"#, team.url, person.id, original.canonical_name).into_bytes();
    let outcome = super::super::roster::capture::parse(&team, body, captured)?;
    let capture = &outcome.capture;
    let roster = outcome.verdict.roster().ok_or("captured roster")?;
    let member = roster.athletes.first().ok_or("captured member")?;
    let published = member.athlete();
    check!(eq; published.name, original.canonical_name);
    check!(eq; published.grad_year, original.grad_year);
    check!(eq; published.gender, original.gender);
    check!(published.outdoor);
    check!(!published.indoor && !published.xc);
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        &published.name,
        published.grad_year,
        published.gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, &published.athlete_id)
            .with_url(&published.profile_url),
    )?;
    let source = SourceRef::new(
        Site::for_jurisdiction(census_domain::UsJurisdiction::Wisconsin).source_id(),
        Some(capture.url.clone()),
    );
    let mut evidence = Evidence::parsed(source.clone(), &capture.fetched_at);
    evidence.note = Some(super::super::roster::capture::row_metadata(
        capture,
        &team,
        &published.athlete_id,
        member.locator(),
    )?);
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: published.grad_year,
        source,
    });
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.evidence.push(evidence);
    store.append(Table::Schools, school)?;
    store.append(Table::Athletes, &athlete)?;
    super::super::roster::retain(store, capture)?;
    Ok(athlete)
}
