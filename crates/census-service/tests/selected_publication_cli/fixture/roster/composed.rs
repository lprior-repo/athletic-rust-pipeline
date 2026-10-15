use super::{
    hash, parse_captured_roster, retain, AthleteId, FetchOutcome, SchoolId, Site, Store, Table,
    TeamRef, TestResult,
};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, PublishedGraduation, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use serde_json::json;

pub(super) fn append(store: &Store, id: &AthleteId) -> TestResult<(AthleteId, SchoolId)> {
    let original = store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|athlete| &athlete.id == id)
        .ok_or("result athlete")?;
    let school = store
        .snapshot()
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|school| school.id == original.school)
        .ok_or("result school")?;
    let owner = school
        .source_identities
        .iter()
        .find(|source| source.namespace == SourceNamespace::MilesplitSchool)
        .ok_or("provider school")?;
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
    let capture = FetchOutcome {
        url: format!("{}/roster", team.url),
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: hash(&body),
        bytes: body.len(),
        fetched_at: "2026-10-01T12:00:00Z".into(),
        from_cache: false,
        content_type: Some("text/html".into()),
        body,
    };
    let verdict =
        parse_captured_roster(std::str::from_utf8(&capture.body)?, team.clone(), &capture)?;
    let member = verdict
        .roster()
        .ok_or("roster")?
        .athletes
        .first()
        .ok_or("roster member")?;
    let proof = member
        .current_roster()
        .ok_or("synthetic scoped current roster assertion")?;
    check!(eq; proof.owner(), team.id.as_str());
    check!(eq; proof.row(), member.locator());
    check!(proof.supports(Sport::OutdoorTrack));
    check!(eq; capture.body.get(proof.heading()), Some(b"<h4>Current Roster</h4>".as_slice()));
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
    check!(eq; &athlete.id, id);
    let row = member.locator();
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("row span overflow")?;
    let span = capture.body.get(row.byte_offset..end).ok_or("row span")?;
    let key = format!(
        "{}#capture={}&row={}&offset={}&length={}",
        capture.url, capture.content_digest, row.ordinal, row.byte_offset, row.byte_length
    );
    let mut evidence = Evidence::parsed(
        SourceRef::new(site.source_id(), Some(capture.url.clone())),
        &capture.fetched_at,
    );
    evidence.note = Some(json!({
        "role": "roster_positive_row_metadata_v1", "source_row_key": key,
        "provider_team_id": team.id, "provider_athlete_id": published.athlete_id, "capture_url": capture.url,
        "response_url": null, "capture_sha256": capture.content_digest, "capture_bytes": capture.bytes,
        "fetched_at": capture.fetched_at, "row": row, "row_sha256": hash(span),
    }).to_string());
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: published.grad_year,
        source: evidence.source.clone(),
    });
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.evidence.push(evidence);
    store.append(Table::Athletes, &athlete)?;
    retain(store, &capture)?;
    Ok((athlete.id, school.id))
}
