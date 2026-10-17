use super::*;
use census_crawl::milesplit::{roster_entities, RosterVerdict, Site};
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1;
use census_crawl::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};

const DIRECTORY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/mi-team-index.html"
));
const ROSTER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/dcc-provider-roster.html"
));
const MANIFEST: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/capture-manifest.json"
));

pub(super) fn install(fixture: &fixture::Fixture) -> TestResult<fixture::Capture> {
    check!(eq; super::super::super::super::hash(DIRECTORY), "39606bf8e07f0170138bb586ae8a762903bb820b515520e89e34745f6c732351");
    check!(eq; super::super::super::super::hash(ROSTER), "246374a87ba6d56802e58e54e4a1edcc5d1eee52106dedb24df185cbd037e178");
    let manifest: Value = serde_json::from_slice(MANIFEST)?;
    let packet = ProviderSchoolPacketV1::new(
        retained(&manifest, "mi-team-index", DIRECTORY, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/provider_school/fixtures/mi-team-index.headers")))?,
        retained(&manifest, "dcc-provider-roster", ROSTER, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/provider_school/fixtures/dcc-provider-roster.headers")))?,
    )?;
    let checked = packet.read()?;
    let RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = checked.roster_verdict()
    else {
        return Err("genuine roster must retain partial disposition".into());
    };
    check!(eq; roster.athletes.len(), 146);
    check!(eq; rejected.len(), 18);
    check!(unfinished.is_none());
    let member = roster
        .athletes
        .iter()
        .find(|row| row.athlete().athlete_id == "13749843")
        .ok_or("genuine Abbott row")?;
    check!(eq; member.athlete().name, "William Abbott");
    check!(eq; member.athlete().grad_year, GradYear::CO2027);
    check!(member.athlete().outdoor);
    let school = checked.school().clone();
    check!(eq; school.name, "Detroit Catholic Central");
    check!(eq; school.city.as_deref(), Some("Novi"));
    check!(eq; school.state, Some(UsJurisdiction::Michigan));
    let year = SchoolYear::new(2026).ok_or("genuine capture school year")?;
    let (_, athletes, teams) = roster_entities(
        &roster.team,
        std::slice::from_ref(member),
        year,
        fixture::AT,
        &Site::for_jurisdiction(UsJurisdiction::Michigan),
    )?;
    let mut athlete = athletes
        .into_iter()
        .next()
        .ok_or("genuine projected Abbott")?;
    let program = teams
        .into_iter()
        .find(|team| team.sport == Sport::OutdoorTrack && team.gender == Gender::Boys)
        .ok_or("genuine outdoor program")?;
    check!(eq; athlete.school, school.id);
    check!(eq; program.school, school.id);
    let row = member.locator();
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("row end overflow")?;
    let span = ROSTER
        .get(row.byte_offset..end)
        .ok_or("original Abbott row")?;
    let digest = packet.roster().body_sha256();
    let url = packet.roster().requested_url();
    let key = format!(
        "{url}#capture={digest}&row={}&offset={}&length={}",
        row.ordinal, row.byte_offset, row.byte_length
    );
    let evidence = athlete
        .evidence
        .first_mut()
        .ok_or("genuine roster evidence")?;
    evidence.source.url = Some(url.to_owned());
    evidence.note = Some(serde_json::to_string(
        &json!({"role":"roster_positive_row_metadata_v1","source_row_key":key,"provider_team_id":"12439","provider_athlete_id":"13749843","capture_url":url,"response_url":url,"capture_sha256":digest,"capture_bytes":ROSTER.len(),"fetched_at":fixture::AT,"row":row,"row_sha256":super::super::super::super::hash(span)}),
    )?);
    fixture.store.append(Table::Schools, &school)?;
    fixture.store.append(Table::Teams, &program)?;
    fixture.store.append(Table::Athletes, &athlete)?;
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
    Ok(fixture::Capture {
        packet,
        school,
        program,
        athlete,
        roster_body: std::str::from_utf8(ROSTER)?.to_owned(),
    })
}

fn retained(
    manifest: &Value,
    label: &str,
    body: &[u8],
    headers: &[u8],
) -> TestResult<RetainedHtmlCaptureV1> {
    let captures = manifest
        .get("captures")
        .and_then(Value::as_array)
        .ok_or("capture manifest")?;
    let capture = captures
        .iter()
        .find(|value| value.get("label").and_then(Value::as_str) == Some(label))
        .ok_or("capture label")?;
    let text = |key| {
        capture
            .get(key)
            .and_then(Value::as_str)
            .ok_or("capture text")
    };
    let acquired = DateTime::parse_from_rfc3339(text("completed_at")?)?.to_utc();
    Ok(RetainedHtmlCaptureV1::new(
        text("requested_url")?,
        text("final_url")?,
        200,
        acquired,
        body,
        RetainedHttpProvenanceV1 {
            request_method: text("request_method")?.into(),
            request_headers: serde_json::from_value(
                capture
                    .get("request_headers")
                    .ok_or("request headers")?
                    .clone(),
            )?,
            response_header_bytes: headers.to_vec(),
            content_type: text("content_type")?.into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(text("started_at")?)?.to_utc(),
        },
    )?)
}

pub(super) fn forged_candidate_for_rejected_original_row(
    fixture: &fixture::Fixture,
    source: &fixture::Capture,
) -> TestResult<CanonicalAthlete> {
    let checked = source.packet.read()?;
    let RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = checked.roster_verdict()
    else {
        return Err("rejected target requires original partial roster".into());
    };
    check!(eq; roster.athletes.len(), 146);
    check!(eq; rejected.len(), 18);
    check!(unfinished.is_none());
    let rejected = rejected
        .iter()
        .find(|row| row.athlete_id.as_deref() == Some("14570860"))
        .ok_or("genuine rejected Arabo identity")?;
    check!(eq; rejected.kind, census_crawl::milesplit::RosterRejectionKind::InvalidGraduationYear);
    check!(!roster
        .athletes
        .iter()
        .any(|row| row.athlete().athlete_id == "14570860"));
    let row = rejected.row;
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("rejected span overflow")?;
    let span = ROSTER
        .get(row.byte_offset..end)
        .ok_or("original rejected span")?;
    let text = std::str::from_utf8(span)?;
    let url = "https://mi.milesplit.com/athletes/14570860-augustine-arabo";
    check!(text.contains(url));
    check!(text.contains("Arabo, Augustine"));
    check!(text.contains("column-grad-year\">0</div>"));
    let identity = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14570860");
    let mut forged = source.athlete.clone();
    forged.canonical_name = "Augustine Arabo".into();
    forged.known_names = vec!["Augustine Arabo".into()];
    forged.grad_year = GradYear::CO2027;
    forged.id = CanonicalAthlete::mint(
        &source.school.id,
        &forged.canonical_name,
        forged.grad_year,
        forged.gender,
        &identity,
    );
    forged.source = Some(identity);
    forged.source_links.clear();
    forged.public_profile_urls = vec![url.into()];
    let evidence = forged
        .evidence
        .first_mut()
        .ok_or("forged positive evidence")?;
    let mut note: Value = serde_json::from_str(
        evidence
            .note
            .as_deref()
            .ok_or("original positive metadata")?,
    )?;
    let capture_url = source.packet.roster().requested_url();
    let digest = source.packet.roster().body_sha256();
    *note.get_mut("source_row_key").ok_or("original row key")? = json!(format!(
        "{capture_url}#capture={digest}&row={}&offset={}&length={}",
        row.ordinal, row.byte_offset, row.byte_length
    ));
    *note
        .get_mut("provider_athlete_id")
        .ok_or("original provider id")? = json!("14570860");
    *note.get_mut("row").ok_or("original locator")? = serde_json::to_value(row)?;
    *note.get_mut("row_sha256").ok_or("original row digest")? =
        json!(super::super::super::super::hash(span));
    evidence.note = Some(serde_json::to_string(&note)?);
    fixture.store.append(Table::Athletes, &forged)?;
    Ok(forged)
}
