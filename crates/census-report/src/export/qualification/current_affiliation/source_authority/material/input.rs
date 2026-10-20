use super::*;
use census_crawl::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use census_crawl::participation_evidence::roster_source_issuance::{
    assess_reviewed_roster_source_historical_v2, assess_reviewed_roster_source_v1,
};

struct Input<'a> {
    proposal: &'a LocallyCheckedRosterIssuanceV1,
    packet: &'a CheckedProviderSchoolPacketV1,
    school: &'a CanonicalSchool,
    body: &'a str,
    metadata: &'a [u8],
    interpretation: &'a [u8],
    qualification: &'a str,
    activation: &'a str,
}

fn read_current_build(
    people: &identity::PersonIndex,
    claim: &ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Capture> {
    let assessment = checked(assess_reviewed_roster_source_v1(claim, grant))?;
    let source = assessment.source(grant)?;
    let objects = source.capture_objects(grant)?;
    let input = Input {
        proposal: assessment.proposal(grant)?,
        packet: source.checked_packet(grant)?,
        school: assessment.school(grant)?,
        body: source.roster_body(grant)?,
        metadata: objects.metadata_bytes(grant)?,
        interpretation: objects.interpretation_bytes(grant)?,
        qualification: assessment.qualification_object(grant)?.0,
        activation: assessment.activation_object(grant)?.0,
    };
    assemble(people, input, grant)
}

pub(in super::super) fn read(
    people: &identity::PersonIndex,
    claim: &ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Capture> {
    let proposal = claim.proposal(grant)?;
    let bytes = claim.object(proposal.capture().interpretation().digest().as_str(), grant)?;
    charge(grant, bytes.len(), 1)?;
    #[derive(serde::Deserialize)]
    struct Version<'a> {
        schema: &'a str,
    }
    let version: Version<'_> = serde_json::from_slice(bytes)
        .map_err(|_| invalid("reviewed roster interpretation version is invalid"))?;
    match version.schema {
        "roster-retained-capture-interpretation-v1" => {
            return read_current_build(people, claim, grant)
        }
        "roster-retained-capture-interpretation-v2"
        | "roster-retained-capture-interpretation-v3" => {}
        _ => return Err(invalid("unsupported reviewed roster interpretation")),
    }
    let assessment = checked(assess_reviewed_roster_source_historical_v2(claim, grant))?;
    let input = Input {
        proposal: assessment.proposal(grant)?,
        packet: assessment.checked_packet(grant)?,
        school: assessment.school(grant)?,
        body: assessment.roster_body(grant)?,
        metadata: assessment.metadata_bytes(grant)?,
        interpretation: assessment.interpretation_bytes(grant)?,
        qualification: assessment.qualification_object(grant)?.0,
        activation: assessment.activation_object(grant)?.0,
    };
    assemble(people, input, grant)
}

fn assemble(
    people: &identity::PersonIndex,
    input: Input<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Capture> {
    encoding::admit(input.proposal, grant)?;
    encoding::admit(input.school, grant)?;
    let copied = input
        .body
        .len()
        .checked_add(input.metadata.len())
        .and_then(|n| n.checked_add(input.interpretation.len()))
        .and_then(|n| n.checked_add(input.qualification.len()))
        .and_then(|n| n.checked_add(input.activation.len()))
        .ok_or(StoreError::CounterOverflow)?;
    charge(grant, copied, 1)?;
    let roster = input
        .packet
        .roster_verdict()
        .roster()
        .ok_or_else(|| invalid("reviewed roster has no complete rows"))?;
    let metadata = checked(SourceCapture::new(
        input.proposal.scope().owner(),
        input.proposal.capture().body().digest().as_str().into(),
        input.proposal.capture().body().bytes().get(),
        input.proposal.capture().observed_day().as_str().into(),
        Some(input.proposal.capture().requested_url().into()),
    ))?;
    let members = roster
        .athletes
        .iter()
        .map(|member| member_record(people, input.proposal, member, grant))
        .collect::<StoreResult<Vec<_>>>()?;
    Ok(Capture {
        source: SourceRef::new(
            "milesplit".to_string(),
            Some(input.proposal.capture().requested_url().into()),
        ),
        owner_span: owners(input.packet, input.proposal)?,
        proposal: input.proposal.clone(),
        school: input.school.clone(),
        body: input.body.to_owned(),
        metadata,
        manifest: input.metadata.to_vec(),
        interpretation: input.interpretation.to_vec(),
        qualification: input.qualification.to_owned(),
        activation: input.activation.to_owned(),
        members,
    })
}
