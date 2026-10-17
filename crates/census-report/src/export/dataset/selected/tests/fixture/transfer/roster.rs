use super::super::roster::retain;
use super::super::{Store, Table, TestResult};
use census_crawl::milesplit::{Site, TeamRef};
use census_domain::model::{AthleteId, SchoolId};
use census_domain::model::{
    CanonicalAthlete, Evidence, PublishedGraduation, SourceIdentity, SourceNamespace, SourceRef,
    Sport,
};

pub(super) fn append(
    store: &Store,
    id: &AthleteId,
    address: super::CurrentAddress,
) -> TestResult<(AthleteId, SchoolId)> {
    let original = store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|athlete| &athlete.id == id)
        .ok_or("result athlete")?;
    let mut school = census_domain::model::CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Synthetic Transfer High",
        "synthetic transfer high",
        Some("Madison"),
    )
    .0;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "52649",
    ));
    if matches!(address, super::CurrentAddress::Retained) {
        super::postal::attach(store, &mut school)?;
    }
    let owner = school
        .source_identities
        .first()
        .ok_or("current school owner")?;
    let person = original.source.as_ref().ok_or("provider athlete")?;
    let site = Site::for_jurisdiction(school.state.ok_or("source jurisdiction")?);
    let team = TeamRef {
        id: owner.id.clone(),
        slug: "synthetic-composed".into(),
        url: format!(
            "https://{}/teams/{}-synthetic-composed",
            site.host(),
            owner.id
        ),
        name: school.name.clone(),
        city_state: site.code().into(),
    };
    let profile = format!(
        "https://{}/athletes/{}-synthetic-composed",
        site.host(),
        person.id
    );
    let body = format!(r#"<link rel="canonical" href="{}/roster"><section><h4>Current Roster</h4><ul id="rosterDataset"><li class="athlete-row data-row"><a href="{profile}">{}</a><div class="column-gender">f</div><div class="column-grad-year">2027</div><div class="data-point" data-season-id="1"><svg class="icon icon-no"></svg></div><div class="data-point" data-season-id="2"><svg class="icon icon-yes"></svg></div><div class="data-point" data-season-id="3"><svg class="icon icon-no"></svg></div></li></ul></section>"#, team.url, original.canonical_name).into_bytes();
    let outcome = super::super::roster::capture::parse(&team, body, "2026-10-01T23:50:00Z")?;
    let capture = &outcome.capture;
    let roster = outcome.verdict.roster().ok_or("captured roster")?;
    let member = roster.athletes.first().ok_or("captured member")?;
    let published = member.athlete();
    check!(!published.indoor);
    check!(published.outdoor);
    check!(!published.xc);
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        &published.name,
        published.grad_year,
        published.gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, &published.athlete_id)
            .with_url(&published.profile_url),
    )?;
    check!(ne; &athlete.id, id);
    let mut evidence = Evidence::parsed(
        SourceRef::new(site.source_id(), Some(capture.url.clone())),
        &capture.fetched_at,
    );
    evidence.note = Some(super::super::roster::capture::row_metadata(
        capture,
        &team,
        &published.athlete_id,
        member.locator(),
    )?);
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: published.grad_year,
        source: evidence.source.clone(),
    });
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.evidence.push(evidence);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    retain(store, capture)?;
    Ok((athlete.id, school.id))
}
