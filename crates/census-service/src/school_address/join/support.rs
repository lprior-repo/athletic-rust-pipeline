use census_domain::model::{CanonicalSchool, Evidence, SourceRef};
use census_domain::school_directory::{IdentifiedKey, LinkRule, ReviewReason, SourceLabel};
use census_domain::UsJurisdiction;
use census_store::{StoreError, StoreResult};

use super::lanes::STATE_ED;
use super::link::{Authority, DirectoryAuthority};
use super::{Counters, LaneEvidence, OutcomeRow};

pub(super) fn parse_key_label(key: &IdentifiedKey) -> String {
    match key {
        IdentifiedKey::Nces(id) => format!("nces:{}", id.as_str()),
        IdentifiedKey::Pss(id) => format!("pss:{}", id.as_str()),
        IdentifiedKey::StateRecord { state, id } => {
            format!("state:{}:{}", state.code(), id.as_str())
        }
    }
}

pub(super) fn rule_name(rule: &LinkRule) -> &'static str {
    match rule {
        LinkRule::ExactName => "exact_name",
        LinkRule::CoreName => "core_name",
        LinkRule::Parenthetical => "parenthetical",
        LinkRule::ParentheticalInner => "parenthetical_inner",
        LinkRule::Alias => "alias",
    }
}

pub(super) fn reason_name(reason: &ReviewReason) -> &'static str {
    match reason {
        ReviewReason::Ambiguous => "ambiguous",
    }
}

pub(super) fn bump_rule(counters: &mut Counters, rule: &LinkRule) -> StoreResult<()> {
    match rule {
        LinkRule::ExactName => bump(&mut counters.exact_name)?,
        LinkRule::CoreName => bump(&mut counters.core_name)?,
        LinkRule::Parenthetical => bump(&mut counters.parenthetical)?,
        LinkRule::ParentheticalInner => bump(&mut counters.parenthetical_inner)?,
        LinkRule::Alias => bump(&mut counters.alias)?,
    }
    Ok(())
}

pub(super) fn bump_review_reason(
    counters: &mut Counters,
    reason: &ReviewReason,
) -> StoreResult<()> {
    match reason {
        ReviewReason::Ambiguous => bump(&mut counters.ambiguous)?,
    }
    Ok(())
}

pub(super) fn row(
    school: &CanonicalSchool,
    state: Option<UsJurisdiction>,
    outcome: &str,
) -> OutcomeRow {
    OutcomeRow {
        school_id: school.id.as_str().to_string(),
        name: school.name.clone(),
        city: school.city.clone(),
        state: state.map(|state| state.code().to_string()),
        outcome: outcome.to_string(),
        rule: None,
        reason: None,
        detail: None,
        candidates: Vec::new(),
    }
}

pub(super) fn bump(counter: &mut u64) -> StoreResult<()> {
    *counter = counter.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    Ok(())
}

pub(super) fn allocation(detail: String) -> StoreError {
    StoreError::Invariant {
        detail: format!("school-address-join allocation failed: {detail}"),
    }
}

pub(super) fn authority_kind(
    key: &IdentifiedKey,
    source: &SourceLabel,
) -> Option<DirectoryAuthority> {
    match (source, key) {
        (SourceLabel::Ccd, IdentifiedKey::Nces(_)) => {
            Some(DirectoryAuthority::Directory("nces-ccd"))
        }
        (SourceLabel::Pss, IdentifiedKey::Pss(_)) => {
            Some(DirectoryAuthority::Directory("nces-pss"))
        }
        (SourceLabel::StateEducationAgency { .. }, IdentifiedKey::StateRecord { .. }) => {
            Some(DirectoryAuthority::Directory(STATE_ED))
        }
        (SourceLabel::AthleticAssociation { .. }, IdentifiedKey::StateRecord { .. }) => {
            Some(DirectoryAuthority::Association)
        }
        _ => None,
    }
}

pub(super) fn owner_id(key: &IdentifiedKey) -> Option<&str> {
    match key {
        IdentifiedKey::Nces(id) => Some(id.as_str()),
        IdentifiedKey::Pss(id) => Some(id.as_str()),
        IdentifiedKey::StateRecord { id, .. } => Some(id.as_str()),
    }
}

pub(super) fn already_owns(school: &CanonicalSchool, authority: &Authority) -> bool {
    school
        .source_identities
        .iter()
        .any(|identity| identity.namespace == authority.namespace && identity.id == authority.id)
}

pub(super) fn lane_evidence(
    authority: &Authority,
    lane: &LaneEvidence,
    url: &str,
    observed_on: &str,
) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(authority.source.clone(), Some(url.to_string())),
        observed_on.to_string(),
    );
    evidence.note = Some(format!(
        "{} lane {} sha256={} generation {}",
        authority.token, lane.path, lane.capture_sha256, lane.generation
    ));
    evidence
}
