use super::*;

pub(in super::super) fn install(
    fixture: &Fixture,
    team: &str,
    person: &str,
    sport: Sport,
    published_owner: bool,
) -> TestResult<Capture> {
    install_padded(fixture, team, person, sport, published_owner, 0)
}

pub(in super::super) fn install_padded(
    fixture: &Fixture,
    team: &str,
    person: &str,
    sport: Sport,
    published_owner: bool,
    padding: usize,
) -> TestResult<Capture> {
    install_row(
        fixture,
        team,
        person,
        sport,
        published_owner,
        padding,
        "Runner, Controlled",
        PhysicalProjection::Retain,
    )
}

pub(in super::super) enum PhysicalProjection {
    Retain,
    Omit,
}

pub(in super::super) fn install_row(
    fixture: &Fixture,
    team: &str,
    person: &str,
    sport: Sport,
    published_owner: bool,
    padding: usize,
    name: &str,
    physical: PhysicalProjection,
) -> TestResult<Capture> {
    let url = format!("https://mi.milesplit.com/teams/{team}-controlled/roster");
    let directory = format!(
        r#"<html><head><meta name="application-name" content="MileSplit Michigan"><link rel="canonical" href="https://mi.milesplit.com/teams"></head><body><table><tr><td><a href="https://mi.milesplit.com/teams/{team}-controlled">Controlled School {team}</a></td><td>Novi, MI, USA</td></tr></table></body></html>"#
    );
    let owner = if published_owner {
        format!(r#"<link rel="canonical" href="{url}">"#)
    } else {
        String::new()
    };
    let season = match sport {
        Sport::IndoorTrack => 1,
        Sport::OutdoorTrack => 2,
        Sport::CrossCountry => 3,
        Sport::Unknown => return Err("controlled sport must be explicit".into()),
    };
    let padding = " ".repeat(padding);
    let body = format!(
        r#"<html><head>{owner}</head><body><section><h4>Current Roster</h4><ul id="rosterDataset"><li class="athlete-row data-row">{padding}<a href="https://mi.milesplit.com/athletes/{person}-controlled">{name}</a><div class="column-gender">m</div><div class="column-grad-year">2027</div><div class="data-point" data-season-id="{season}"><svg class="icon icon-yes"></svg></div></li></ul></section></body></html>"#
    );
    let packet = ProviderSchoolPacketV1::new(
        retained("https://mi.milesplit.com/teams", &directory)?,
        retained(&url, &body)?,
    )?;
    let checked = packet.read()?;
    let school = checked.school().clone();
    let year = SchoolYear::new(2026).ok_or("controlled school year")?;
    let program = CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, sport, Gender::Boys, year),
        school: school.id.clone(),
        sport,
        gender: Gender::Boys,
        school_year: year,
        level: None,
        source_identities: vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, team)],
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    let roster = checked
        .roster_verdict()
        .roster()
        .ok_or("controlled parsed roster")?;
    let team_ref = TeamRef {
        id: team.into(),
        slug: "controlled".into(),
        url: url.trim_end_matches("/roster").into(),
        name: school.name.clone(),
        city_state: "Novi, MI".into(),
    };
    let (_, athletes, _) = roster_entities(
        &team_ref,
        &roster.athletes,
        year,
        AT,
        &Site::for_jurisdiction(UsJurisdiction::Michigan),
    )?;
    let mut athlete = athletes
        .into_iter()
        .next()
        .ok_or("controlled parsed athlete")?;
    athlete.school = school.id.clone();
    let member = roster.athletes.first().ok_or("controlled located member")?;
    let row = member.locator();
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("controlled row overflow")?;
    let span = body
        .as_bytes()
        .get(row.byte_offset..end)
        .ok_or("controlled row bytes")?;
    let digest = super::super::hash(body.as_bytes());
    let key = format!(
        "{url}#capture={digest}&row={}&offset={}&length={}",
        row.ordinal, row.byte_offset, row.byte_length
    );
    let evidence = athlete
        .evidence
        .first_mut()
        .ok_or("controlled roster evidence")?;
    evidence.source.url = Some(url.clone());
    evidence.note = Some(serde_json::to_string(
        &json!({"role":"roster_positive_row_metadata_v1","source_row_key":key,"provider_team_id":team,"provider_athlete_id":person,"capture_url":url,"response_url":url,"capture_sha256":digest,"capture_bytes":body.len(),"fetched_at":AT,"row":row,"row_sha256":super::super::hash(span)}),
    )?);
    fixture.store.append(Table::Schools, &school)?;
    fixture.store.append(Table::Teams, &program)?;
    if matches!(physical, PhysicalProjection::Retain) {
        fixture.store.append(Table::Athletes, &athlete)?;
    }
    fixture.store.append(
        Table::SourceObservations,
        &SourceObservation::School(checked.observation().clone()),
    )?;
    let schema = census_store::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    fixture.store.journal_done(
        schema.phase(),
        packet.reference()?.as_str(),
        &serde_json::to_value(&packet)?,
    )?;
    Ok(Capture {
        packet,
        school,
        program,
        athlete,
    })
}

fn retained(url: &str, body: &str) -> TestResult<RetainedHtmlCaptureV1> {
    let at = instant()?;
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        at,
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: Vec::new(),
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: at,
        },
    )?)
}
