use super::*;
use crate::milesplit::{LocatedRosterAthlete, RosterVerdict};

pub(super) fn inventory(
    checked: &CheckedProviderSchoolPacketV1,
    parser_manifest: &[u8],
    metadata: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::Inventory> {
    let RosterVerdict::Complete { roster } = checked.roster_verdict() else {
        return Err(invalid(
            "semantic inventory requires complete checked roster",
        ));
    };
    grant.charge_steps(2).map_err(domain)?;
    grant.charge_bytes(256).map_err(domain)?;
    budget::charge(checked.school(), grant)?;
    budget::charge(checked.observation(), grant)?;
    budget::charge(checked.directory_proof(), grant)?;
    budget::charge(&roster.team, grant)?;
    budget::charge(&checked.reference(), grant)?;
    grant.charge_bytes(metadata.len()).map_err(domain)?;
    grant.charge_bytes(parser_manifest.len()).map_err(domain)?;
    let (kernel_text, kernel_digest) = kernel::compiled_identity()?;
    let kernel_bytes = kernel_text.len();
    grant.charge_bytes(kernel_bytes).map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    let mut members = Vec::new();
    for member in &roster.athletes {
        grant.charge_steps(1).map_err(domain)?;
        grant
            .charge_bytes(std::mem::size_of::<wire::Member>())
            .map_err(domain)?;
        members
            .try_reserve_exact(1)
            .map_err(|_| invalid("semantic member allocation refused"))?;
        members.push(row(member, grant)?);
    }
    Ok(wire::Inventory {
        schema: SCHEMA.into(),
        interpretation: INTERPRETATION.into(),
        provider_semantic_contract:
            crate::participation_evidence::provider_school::PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1
                .into(),
        kernel_document: std::borrow::Cow::Borrowed(kernel_text),
        kernel_digest: kernel_digest.to_string(),
        kernel_bytes: u64::try_from(kernel_bytes)
            .map_err(|_| census_store::StoreError::CounterOverflow)?,
        packet: checked.reference().as_str().into(),
        parser_manifest: hash(parser_manifest),
        metadata: hash(metadata),
        school: checked.school().clone(),
        observation: checked.observation().clone(),
        directory: directory(checked.directory_proof()),
        team: roster.team.clone(),
        owners: statements::owners(checked, grant)?,
        members,
    })
}

fn row(
    row: &LocatedRosterAthlete,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::Member> {
    let value = row.athlete();
    budget::charge(
        &(
            &value.roster_name,
            &value.name,
            &value.athlete_id,
            &value.profile_url,
        ),
        grant,
    )?;
    Ok(wire::Member {
        roster_name: value.roster_name.clone(),
        name: value.name.clone(),
        gender: value.gender,
        grad_year: value.grad_year,
        provider_id: value.athlete_id.clone(),
        profile_url: value.profile_url.clone(),
        sports: [value.indoor, value.outdoor, value.xc],
        row: row.locator(),
        current: statements::current(row, grant)?,
    })
}

fn directory(
    value: &crate::participation_evidence::provider_school::ProviderSchoolDirectoryProofV1,
) -> wire::Directory {
    wire::Directory {
        capture: value.capture_sha256().into(),
        acquired_at: value.acquired_at(),
        disposition: value.disposition(),
        parsed_teams: value.parsed_teams(),
        errors: value.errors(),
        unfinished: value.unfinished().to_vec(),
        row: value.row(),
        row_digest: value.row_sha256().into(),
        name: value.name(),
        city: value.city(),
        anchor: value.anchor(),
    }
}
