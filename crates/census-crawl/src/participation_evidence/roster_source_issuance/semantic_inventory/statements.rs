use super::*;
use crate::milesplit::{
    LocatedRosterAthlete, PublishedRosterOwnerKindV1, PublishedRosterOwnerTerminationV1,
    RosterTemporalUnavailable, RosterWitnessUnavailableV1,
};
use census_domain::model::Sport;

pub(super) fn current(
    value: &LocatedRosterAthlete,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::Current> {
    let Some(current) = value.current_roster() else {
        return Ok(wire::Current::Unavailable {
            reason: temporal(
                value
                    .temporal_unavailability()
                    .ok_or_else(|| invalid("missing roster temporal outcome"))?,
            ),
        });
    };
    budget::charge(&(current.owner(), current.member()), grant)?;
    let statements = match current.source_statements_v1() {
        Ok(value) => wire::Statements::Available {
            person: value.person().range().into(),
            membership: value.membership().range().into(),
            temporal: value.temporal().range().into(),
        },
        Err(reason) => wire::Statements::Unavailable {
            reason: witness(reason),
        },
    };
    let owner = compact_str(current.owner())?;
    let member = compact_str(current.member())?;
    Ok(wire::Current::Available {
        owner,
        member,
        row: current.row(),
        section: current.section().into(),
        container: current.container().into(),
        heading: current.heading().into(),
        heading_digest: hex(current.heading_sha256()),
        sports: [
            current.supports(Sport::IndoorTrack),
            current.supports(Sport::OutdoorTrack),
            current.supports(Sport::CrossCountry),
        ],
        statements,
    })
}

pub(super) fn owners(
    value: &CheckedProviderSchoolPacketV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::Owners> {
    let owners = match value.published_roster_owners_v1() {
        Ok(value) => value,
        Err(reason) => {
            return Ok(wire::Owners::Unavailable {
                reason: witness(reason),
            })
        }
    };
    let mut declarations = Vec::new();
    for owner in owners.declarations() {
        grant.charge_steps(1).map_err(domain)?;
        grant
            .charge_bytes(std::mem::size_of::<wire::Owner>())
            .map_err(domain)?;
        declarations
            .try_reserve_exact(1)
            .map_err(|_| invalid("semantic owner allocation refused"))?;
        declarations.push(wire::Owner {
            provider_id: owner.provider_id(),
            jurisdiction: owner.jurisdiction(),
            span: owner.span().range().into(),
            kind: match owner.kind() {
                PublishedRosterOwnerKindV1::Canonical => wire::OwnerKind::Canonical,
                PublishedRosterOwnerKindV1::OpenGraph => wire::OwnerKind::OpenGraph,
                PublishedRosterOwnerKindV1::Twitter => wire::OwnerKind::Twitter,
                PublishedRosterOwnerKindV1::JsonLdUrl => wire::OwnerKind::JsonLdUrl,
                PublishedRosterOwnerKindV1::JsonLdId => wire::OwnerKind::JsonLdId,
            },
            termination: match owner.termination() {
                PublishedRosterOwnerTerminationV1::Tag => wire::OwnerTermination::Tag,
                PublishedRosterOwnerTerminationV1::ScriptEnd => wire::OwnerTermination::ScriptEnd,
                PublishedRosterOwnerTerminationV1::ScriptEof => wire::OwnerTermination::ScriptEof,
            },
        });
    }
    Ok(wire::Owners::Available { declarations })
}

fn temporal(value: RosterTemporalUnavailable) -> wire::TemporalGap {
    match value {
        RosterTemporalUnavailable::MissingStatement => wire::TemporalGap::MissingStatement,
        RosterTemporalUnavailable::AmbiguousStatement => wire::TemporalGap::AmbiguousStatement,
        RosterTemporalUnavailable::MalformedMarkup => wire::TemporalGap::MalformedMarkup,
        RosterTemporalUnavailable::ResourceLimit => wire::TemporalGap::ResourceLimit,
        RosterTemporalUnavailable::InactiveMember => wire::TemporalGap::InactiveMember,
    }
}

fn witness(value: RosterWitnessUnavailableV1) -> wire::WitnessGap {
    match value {
        RosterWitnessUnavailableV1::MissingPublishedOwner => {
            wire::WitnessGap::MissingPublishedOwner
        }
        RosterWitnessUnavailableV1::MissingSourceSpan => wire::WitnessGap::MissingSourceSpan,
        RosterWitnessUnavailableV1::SourceSpanLimit => wire::WitnessGap::SourceSpanLimit,
        RosterWitnessUnavailableV1::OwnerInventoryLimit => wire::WitnessGap::OwnerInventoryLimit,
        RosterWitnessUnavailableV1::ResourceLimit => wire::WitnessGap::ResourceLimit,
    }
}

fn compact_str(value: &str) -> StoreResult<Box<str>> {
    let mut s = String::new();
    s.try_reserve_exact(value.len())
        .map_err(|_| invalid("semantic inventory text allocation refused"))?;
    s.push_str(value);
    Ok(s.into_boxed_str())
}

fn hex(value: &[u8; 32]) -> String {
    value
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 15])
        .map(|digit| {
            char::from(if digit < 10 {
                b'0' + digit
            } else {
                b'a' + digit - 10
            })
        })
        .collect()
}
