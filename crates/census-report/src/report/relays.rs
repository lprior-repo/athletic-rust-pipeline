use super::{Derivation, EventAdmission, ReportResult};
use crate::grouping::{Admission, GroupingError};
use census_domain::model::{
    EventDisposition, EventKind, EvidenceMethod, Mark, RelayId, RelayResult,
};
use serde::Serialize;
use std::collections::HashSet;

const STAGE: &str = "recruiting relay projection";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayGap {
    MissingEvent,
    NonRelayEvent,
    MismatchedMeet,
    MissingTeam,
    MissingSourceOwnership,
    ContradictoryTeamContext,
    ConflictedEvidence,
    InvalidPublishedResult,
    InvalidMemberOrder,
    OutsideEvidenceScope,
    NoRequestedCohortMember,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayDisposition<'a> {
    Included,
    Event(&'a EventDisposition),
    Unresolved(RelayGap),
}

#[derive(Debug, Serialize)]
pub struct RelayAdmission<'a> {
    pub relay: &'a RelayId,
    pub disposition: RelayDisposition<'a>,
}

#[derive(Debug, Serialize)]
pub struct RelaySelection<'a> {
    pub event_kind: &'a EventKind,
    pub result: &'a RelayResult,
}

pub struct RelayProjection<'a> {
    pub selections: Vec<RelaySelection<'a>>,
    pub admissions: Vec<RelayAdmission<'a>>,
}

impl<'a> RelayProjection<'a> {
    pub fn of(
        derivation: &Derivation<'_>,
        admission: &'a EventAdmission<'a>,
    ) -> ReportResult<Self> {
        admission.ensure_dataset(derivation.dataset())?;
        let cohort = cohort(derivation)?;
        let meets = eligible_meets(derivation)?;
        let mut selections = Vec::new();
        let mut admissions = Vec::new();
        let rows = &admission.dataset().relays;
        reserve(rows.len(), std::mem::size_of::<RelaySelection<'_>>())?;
        reserve(rows.len(), std::mem::size_of::<RelayAdmission<'_>>())?;
        selections.try_reserve(rows.len()).map_err(allocation)?;
        admissions.try_reserve(rows.len()).map_err(allocation)?;
        let mut budget = Admission::new(STAGE);
        for relay in rows {
            budget.record(relay)?;
            let members = relay.members.len();
            let work = members
                .checked_mul(members)
                .ok_or(GroupingError::Overflow {
                    stage: STAGE,
                    resource: "published member order work",
                })?;
            budget.charge(0, work)?;
            let disposition = disposition(relay, derivation, admission, &cohort, &meets);
            if matches!(disposition, RelayDisposition::Included) {
                if let Some(event_kind) = admission.included_kind(&relay.event) {
                    selections.push(RelaySelection {
                        event_kind,
                        result: relay,
                    });
                }
            }
            admissions.push(RelayAdmission {
                relay: &relay.id,
                disposition,
            });
        }
        selections.sort_by(|left, right| left.result.id.cmp(&right.result.id));
        admissions.sort_by(|left, right| left.relay.cmp(right.relay));
        Ok(Self {
            selections,
            admissions,
        })
    }
}

fn cohort<'d>(derivation: &'d Derivation<'_>) -> ReportResult<HashSet<&'d str>> {
    let mut cohort = HashSet::new();
    reserve(derivation.athletes().len(), std::mem::size_of::<&str>())?;
    cohort
        .try_reserve(derivation.athletes().len())
        .map_err(allocation)?;
    cohort.extend(
        derivation
            .athletes()
            .iter()
            .map(|athlete| athlete.id.as_str()),
    );
    Ok(cohort)
}

fn eligible_meets<'d>(derivation: &'d Derivation<'_>) -> ReportResult<HashSet<&'d str>> {
    let mut meets = HashSet::new();
    let rows = derivation.dataset().meets.len();
    reserve(rows, std::mem::size_of::<&str>())?;
    meets.try_reserve(rows).map_err(allocation)?;
    meets.extend(derivation.historical_meets().map(|meet| meet.id.as_str()));
    Ok(meets)
}

fn disposition<'a>(
    relay: &RelayResult,
    derivation: &Derivation<'_>,
    admission: &'a EventAdmission<'a>,
    cohort: &HashSet<&str>,
    meets: &HashSet<&str>,
) -> RelayDisposition<'a> {
    let unresolved = RelayDisposition::Unresolved;
    let Some(disposition) = admission.disposition(&relay.event) else {
        return unresolved(RelayGap::MissingEvent);
    };
    let kind = match disposition {
        EventDisposition::Included(kind) => kind,
        EventDisposition::Excluded(_) | EventDisposition::Unresolved(_) => {
            return RelayDisposition::Event(disposition);
        }
    };
    if !kind.is_relay() {
        return unresolved(RelayGap::NonRelayEvent);
    }
    let Some(event) = admission.event(&relay.event) else {
        return unresolved(RelayGap::MissingEvent);
    };
    if event.meet != relay.meet {
        return unresolved(RelayGap::MismatchedMeet);
    }
    if let Some(gap) = structural_gap(relay, admission) {
        return unresolved(gap);
    }
    if derivation.scope().primary_evidence(relay).is_none()
        || derivation.scope().primary_evidence(event).is_none()
        || !meets.contains(relay.meet.as_str())
    {
        return unresolved(RelayGap::OutsideEvidenceScope);
    }
    if !relay
        .members
        .iter()
        .filter_map(|member| member.athlete.as_ref())
        .any(|athlete| {
            let canonical = derivation.athlete_aliases().get(athlete.as_str());
            cohort.contains(canonical.map_or(athlete.as_str(), String::as_str))
        })
    {
        return unresolved(RelayGap::NoRequestedCohortMember);
    }
    RelayDisposition::Included
}

fn structural_gap(relay: &RelayResult, admission: &EventAdmission<'_>) -> Option<RelayGap> {
    let Some(team) = admission.dataset().teams.get(&relay.team) else {
        return Some(RelayGap::MissingTeam);
    };
    let Some(event) = admission.event(&relay.event) else {
        return Some(RelayGap::MissingEvent);
    };
    let Some(sport) = admission.sport(&relay.event) else {
        return Some(RelayGap::MissingEvent);
    };
    if team.sport != sport || team.gender != event.gender {
        return Some(RelayGap::ContradictoryTeamContext);
    }
    if !relay.retained_conflicts.is_empty() || !team.retained_conflicts.is_empty() {
        return Some(RelayGap::ConflictedEvidence);
    }
    if relay.source_key.trim().is_empty()
        || relay.source_team.as_ref().is_none_or(|owner| {
            owner.id.trim().is_empty() || !team.source_identities.contains(owner)
        })
        || !relay.evidence.iter().any(|evidence| {
            evidence.method == EvidenceMethod::Parsed
                && !evidence.source.id.trim().is_empty()
                && evidence
                    .source
                    .url
                    .as_ref()
                    .is_some_and(|url| !url.trim().is_empty())
        })
    {
        return Some(RelayGap::MissingSourceOwnership);
    }
    if !matches!(relay.mark, Mark::TimeSeconds(_)) {
        return Some(RelayGap::InvalidPublishedResult);
    }
    if relay.members.is_empty()
        || relay
            .members
            .iter()
            .any(|member| member.name_as_published.trim().is_empty())
        || relay.members.iter().enumerate().any(|(index, member)| {
            relay
                .members
                .iter()
                .take(index)
                .any(|previous| previous.order == member.order)
        })
    {
        return Some(RelayGap::InvalidMemberOrder);
    }
    None
}

fn allocation(_: std::collections::TryReserveError) -> GroupingError {
    GroupingError::Allocation { stage: STAGE }
}

fn reserve(rows: usize, item_bytes: usize) -> ReportResult<()> {
    let bytes = rows
        .checked_mul(item_bytes)
        .and_then(|bytes| bytes.checked_mul(4))
        .ok_or(GroupingError::Overflow {
            stage: STAGE,
            resource: "relay scratch bytes",
        })?;
    Admission::new(STAGE).charge(bytes, rows)?;
    Ok(())
}
