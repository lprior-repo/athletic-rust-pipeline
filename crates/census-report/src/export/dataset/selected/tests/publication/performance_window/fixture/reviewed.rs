use super::super::TestResult;
use census_crawl::milesplit::{LocatedRosterAthlete, Site};
use census_crawl::participation_evidence::{
    provider_school::ProviderSchoolPacketV1, RetainedBodyEncodingV1, RetainedHtmlCaptureV1,
    RetainedHttpProvenanceV1,
};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Evidence, Gender,
    PublishedGraduation, SchoolYear, SourceIdentity, SourceNamespace, SourceObservation, SourceRef,
    Sport,
};
use census_store::{Store, Table};
use serde_json::json;
use sha2::{Digest, Sha256};

mod issuance;
pub(super) use issuance::review;

pub(super) const AT: &str = "2026-10-01T12:00:00Z";
const ALIAS_NAME: &str = "Controlled Synthetic PR Alias";
const ALIAS_PERSON: &str = "990000118";

pub(super) struct Capture {
    packet: ProviderSchoolPacketV1,
    school: CanonicalSchool,
    program: CanonicalTeam,
    body: String,
}

pub(super) fn append(
    store: &Store,
    original: &AthleteId,
) -> TestResult<(Capture, CanonicalAthlete)> {
    let original = store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|row| &row.id == original)
        .ok_or("fresh result subject")?;
    let (packet, body) = packet(&original)?;
    let checked = packet.read()?;
    let school = checked.school().clone();
    check!(eq; school.id, original.school);
    let program = program(&school)?;
    let roster = checked
        .roster_verdict()
        .roster()
        .ok_or("fresh complete roster")?;
    check!(eq; roster.athletes.len(), 2);
    let original_row = athlete(
        &school,
        roster.athletes.first().ok_or("original row")?,
        original.source.clone().ok_or("original primary")?,
        evidence(
            &packet,
            &body,
            roster.athletes.first().ok_or("original row")?,
        )?,
    )?;
    check!(eq; original_row.id, original.id);
    let alias_member = roster.athletes.get(1).ok_or("alias row")?;
    let alias = athlete(
        &school,
        alias_member,
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "990000117"),
        evidence(&packet, &body, alias_member)?,
    )?;
    store.append(Table::Schools, &school)?;
    store.append(Table::Teams, &program)?;
    store.append(Table::Athletes, &original_row)?;
    store.append(
        Table::SourceObservations,
        &SourceObservation::School(checked.observation().clone()),
    )?;
    let schema = census_store::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    store.journal_done(
        schema.phase(),
        packet.reference()?.as_str(),
        &serde_json::to_value(&packet)?,
    )?;
    let snapshot = store.snapshot();
    let school = snapshot
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|row| row.id == school.id)
        .ok_or("fresh merged school projection")?;
    let program = snapshot
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .find(|row| row.id == program.id)
        .ok_or("fresh merged program projection")?;
    Ok((
        Capture {
            packet,
            school,
            program,
            body,
        },
        alias,
    ))
}

fn packet(original: &CanonicalAthlete) -> TestResult<(ProviderSchoolPacketV1, String)> {
    let url = "https://mi.milesplit.com/teams/38332-controlled-pr/roster";
    let directory = r#"<html><head><meta name="application-name" content="MileSplit Michigan"><link rel="canonical" href="https://mi.milesplit.com/teams"></head><body><table><tr><td><a href="https://mi.milesplit.com/teams/38332-controlled-pr">Detroit Catholic Central</a></td><td>Novi, MI, USA</td></tr></table></body></html>"#;
    let body = format!(
        r#"<html><head><link rel="canonical" href="{url}"></head><body><section><h4>Current Roster</h4><ul id="rosterDataset">{}{}</ul></section></body></html>"#,
        row("14222592", &original.canonical_name),
        row(ALIAS_PERSON, ALIAS_NAME),
    );
    let packet = ProviderSchoolPacketV1::new(
        retained("https://mi.milesplit.com/teams", directory)?,
        retained(url, &body)?,
    )?;
    Ok((packet, body))
}

fn program(school: &CanonicalSchool) -> TestResult<CanonicalTeam> {
    let year = SchoolYear::new(2026).ok_or("fresh roster year")?;
    Ok(CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::OutdoorTrack, Gender::Boys, year),
        school: school.id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: year,
        level: None,
        source_identities: vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, "38332")],
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    })
}

fn row(person: &str, name: &str) -> String {
    format!(
        r#"<li class="athlete-row data-row"><a href="https://mi.milesplit.com/athletes/{person}-controlled-pr">{name}</a><div class="column-gender">m</div><div class="column-grad-year">2027</div><div class="data-point" data-season-id="1"><svg class="icon icon-no"></svg></div><div class="data-point" data-season-id="2"><svg class="icon icon-yes"></svg></div><div class="data-point" data-season-id="3"><svg class="icon icon-no"></svg></div></li>"#
    )
}

fn athlete(
    school: &CanonicalSchool,
    member: &LocatedRosterAthlete,
    primary: SourceIdentity,
    evidence: Evidence,
) -> TestResult<CanonicalAthlete> {
    let source = member.athlete();
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        &source.name,
        source.grad_year,
        source.gender,
        primary,
    )?;
    athlete.add_identity(
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, &source.athlete_id)
            .with_url(&source.profile_url),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: source.grad_year,
        source: evidence.source.clone(),
    });
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.evidence.push(evidence);
    Ok(athlete)
}

fn evidence(
    packet: &ProviderSchoolPacketV1,
    body: &str,
    member: &LocatedRosterAthlete,
) -> TestResult<Evidence> {
    let row = member.locator();
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("fresh row overflow")?;
    let span = body
        .as_bytes()
        .get(row.byte_offset..end)
        .ok_or("fresh row span")?;
    let url = packet.roster().requested_url();
    let digest = packet.roster().body_sha256();
    let key = format!(
        "{url}#capture={digest}&row={}&offset={}&length={}",
        row.ordinal, row.byte_offset, row.byte_length,
    );
    let mut evidence = Evidence::parsed(
        SourceRef::new(
            Site::for_jurisdiction(census_domain::UsJurisdiction::Michigan).source_id(),
            Some(url.into()),
        ),
        AT,
    );
    evidence.note = Some(
        json!({
            "role":"roster_positive_row_metadata_v1", "source_row_key":key,
            "provider_team_id":"38332", "provider_athlete_id":member.athlete().athlete_id,
            "capture_url":url, "response_url":url, "capture_sha256":digest,
            "capture_bytes":body.len(), "fetched_at":AT, "row":row, "row_sha256":hash(span),
        })
        .to_string(),
    );
    Ok(evidence)
}

fn retained(url: &str, body: &str) -> TestResult<RetainedHtmlCaptureV1> {
    let at = chrono::DateTime::parse_from_rfc3339(AT)?.to_utc();
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

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
