use super::super::{invalid, RetainedHtmlCaptureV1};
use super::directory::Selected;
use super::{
    CheckedProviderSchoolPacketV1, ProviderSchoolPacketRefV1, ProviderSchoolPacketV1,
    PROVIDER_SCHOOL_INTAKE_PHASE,
};
use crate::milesplit::{parse_captured_roster, roster_entities, Site};
use crate::net::FetchOutcome;
use crate::CrawlResult;
use census_domain::model::{
    Evidence, SchoolYear, SourceNamespace, SourceRef, SourceSchoolObservation,
};
use census_store::StoreResult;
use serde::Serialize;

pub(super) fn read(
    packet: &ProviderSchoolPacketV1,
    reference: ProviderSchoolPacketRefV1,
    selected: Selected,
) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    let body = packet.roster.body()?;
    let fetch = fetch(packet.roster(), &body)?;
    let roster_verdict = parse_captured_roster(&body, selected.team.clone(), &fetch)?;
    let date = packet.roster.acquired_at().format("%Y-%m-%d").to_string();
    let year = SchoolYear::from_date(&date)
        .ok_or_else(|| invalid("provider school acquisition year unsupported"))?;
    let at = packet.roster.acquired_at().to_rfc3339();
    let (mut school, _, _) = roster_entities(&selected.team, &[], year, &at, &selected.site)?;
    school.evidence.clear();
    school
        .evidence
        .try_reserve(2)
        .map_err(|_| invalid("provider school evidence allocation refused"))?;
    school.evidence.push(evidence(
        packet.directory(),
        &selected.site,
        &reference,
        "directory_name_city",
        Some(&selected.proof),
    )?);
    school.evidence.push(evidence(
        packet.roster(),
        &selected.site,
        &reference,
        "provider_roster_owner",
        None,
    )?);
    let mut observation = SourceSchoolObservation::of_school(
        &SourceNamespace::MilesplitSchool,
        &school,
        &packet.directory.acquired_at().to_rfc3339(),
    )
    .ok_or_else(|| invalid("provider school projection has no source identity"))?;
    observation.source_row_key = format!(
        "{}:{}:directory:{}:{}:{}",
        PROVIDER_SCHOOL_INTAKE_PHASE,
        reference.as_str(),
        selected.proof.row.ordinal,
        selected.proof.row.byte_offset,
        selected.proof.row.byte_length
    );
    Ok(CheckedProviderSchoolPacketV1 {
        reference,
        school,
        observation,
        roster_verdict,
        directory_proof: selected.proof,
    })
}

fn fetch(capture: &RetainedHtmlCaptureV1, body: &str) -> StoreResult<FetchOutcome> {
    let mut original = Vec::new();
    original
        .try_reserve_exact(body.len())
        .map_err(|_| invalid("provider school original allocation refused"))?;
    original.extend_from_slice(body.as_bytes());
    Ok(FetchOutcome {
        url: capture.requested_url().into(),
        response_url: Some(capture.requested_url().into()),
        method: capture.provenance().request_method.clone(),
        status: 200,
        content_digest: capture.body_sha256().into(),
        bytes: body.len(),
        fetched_at: capture.acquired_at().to_rfc3339(),
        from_cache: false,
        content_type: Some(capture.provenance().content_type.clone()),
        body: original,
    })
}

#[derive(Serialize)]
struct Provenance<'a> {
    role: &'static str,
    phase: &'static str,
    packet: &'a str,
    capture_sha256: &'a str,
    directory_fields: Option<&'a super::ProviderSchoolDirectoryProofV1>,
}

fn evidence(
    capture: &RetainedHtmlCaptureV1,
    site: &Site,
    reference: &ProviderSchoolPacketRefV1,
    role: &'static str,
    directory: Option<&super::ProviderSchoolDirectoryProofV1>,
) -> StoreResult<Evidence> {
    let mut evidence = Evidence::parsed(
        SourceRef::new(site.source_id(), Some(capture.requested_url().into())),
        capture.acquired_at().to_rfc3339(),
    );
    let provenance = Provenance {
        role,
        phase: PROVIDER_SCHOOL_INTAKE_PHASE,
        packet: reference.as_str(),
        capture_sha256: capture.body_sha256(),
        directory_fields: directory,
    };
    let bytes = super::super::codec::encode(&provenance, 8192)?;
    evidence.note = Some(
        String::from_utf8(bytes)
            .map_err(|_| invalid("provider school evidence encoding invalid"))?,
    );
    Ok(evidence)
}
