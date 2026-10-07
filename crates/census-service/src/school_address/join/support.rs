use census_domain::model::{
    CanonicalSchool, Evidence, SchoolAddressError, SchoolPostalAddress, SourceIdentity,
    SourceNamespace, SourceRef,
};
use census_domain::school_directory::{
    IdentifiedKey, LinkMatch, LinkRule, ReviewReason, SourceLabel,
};
use census_domain::UsJurisdiction;
use census_store::{StoreError, StoreResult};

use super::lanes::STATE_ED;
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DirectoryAuthority {
    Directory(&'static str),
    Association,
}

pub(super) struct Authority {
    pub(super) token: String,
    pub(super) source: String,
    pub(super) namespace: SourceNamespace,
    pub(super) id: String,
}

#[derive(Clone, Copy)]
pub(super) struct Target<'a> {
    pub(super) school: &'a CanonicalSchool,
    pub(super) state: UsJurisdiction,
    pub(super) matched: &'a LinkMatch,
}

pub(super) fn postal_claim(
    matched: &LinkMatch,
    authority: &Authority,
    lane: &LaneEvidence,
    url: &str,
    evidence: &Evidence,
) -> Option<Result<SchoolPostalAddress, SchoolAddressError>> {
    let address = matched.address.clone()?;
    Some(SchoolPostalAddress::new(
        address,
        SourceIdentity::new(authority.namespace.clone(), authority.id.as_str()).with_url(url),
        matched.source.clone(),
        evidence.clone(),
        lane.capture_sha256.clone(),
    ))
}

pub(super) fn retain_evidence(
    clone: &mut CanonicalSchool,
    evidence: &Evidence,
) -> StoreResult<bool> {
    if clone.evidence.contains(evidence) {
        return Ok(false);
    }
    clone
        .evidence
        .try_reserve(1)
        .map_err(|error| allocation(error.to_string()))?;
    clone.evidence.push(evidence.clone());
    Ok(true)
}

pub(super) fn stamp_website(
    target: Target<'_>,
    clone: &mut CanonicalSchool,
    evidence: &Evidence,
    counters: &mut Counters,
) -> StoreResult<bool> {
    let Some(website) = target
        .matched
        .website
        .clone()
        .filter(|_| target.school.school_website.is_none())
    else {
        return Ok(false);
    };
    attach_website(counters, clone, website, evidence)?;
    Ok(true)
}

fn attach_website(
    counters: &mut Counters,
    clone: &mut CanonicalSchool,
    website: String,
    evidence: &Evidence,
) -> StoreResult<()> {
    clone
        .evidence
        .try_reserve(1)
        .map_err(|error| allocation(error.to_string()))?;
    clone.school_website = Some(website);
    clone.evidence.push(evidence.clone());
    bump(&mut counters.websites)
}
