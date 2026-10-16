use super::{invalid, inventory, resolved_id, PublicIdentityGroup};
use crate::report::ReportResult;
use census_domain::model::{AthleteCandidateId, CanonicalAthlete};
use census_domain::public_identity_history::{PublicResolution, ResolvedSuccessor};
use census_store::AcceptedPublicIdentitySnapshot;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) struct Partition {
    pub(super) groups: BTreeMap<AthleteCandidateId, PublicIdentityGroup>,
    pub(super) candidates: BTreeMap<AthleteCandidateId, AthleteCandidateId>,
    pub(super) aliases: HashMap<String, String>,
}

pub(super) fn capture(
    accepted: &AcceptedPublicIdentitySnapshot,
    athletes: &[CanonicalAthlete],
) -> ReportResult<Partition> {
    let inventory = inventory(athletes)?;
    let mut partition = Partition {
        groups: BTreeMap::new(),
        candidates: BTreeMap::new(),
        aliases: HashMap::new(),
    };
    inventory.values().try_for_each(|candidate| {
        let resolution = accepted.resolve_candidate(candidate)?;
        let canonical = match successor(&resolution) {
            Some(resolved) => resolved.representative_candidate.clone(),
            None => candidate.clone(),
        };
        let public_id = resolved_id(&resolution).cloned();
        let group = partition
            .groups
            .entry(canonical.clone())
            .or_insert_with(|| PublicIdentityGroup {
                canonical_candidate: canonical.clone(),
                public_id: public_id.clone(),
                members: BTreeMap::new(),
            });
        if group.public_id != public_id {
            return Err(invalid(
                "retained public partition assigns incompatible groups to one representative",
            ));
        }
        if public_id.is_some() && candidate != &canonical {
            partition
                .aliases
                .insert(candidate.to_string(), canonical.to_string());
        }
        group.members.insert(candidate.clone(), resolution);
        partition.candidates.insert(candidate.clone(), canonical);
        Ok::<_, crate::report::ReportError>(())
    })?;
    partition
        .groups
        .values()
        .try_fold(BTreeSet::new(), |mut seen, group| {
            require_representative(group)?;
            if group.public_id().is_some_and(|id| !seen.insert(id)) {
                return Err(invalid(
                    "retained public partition repeats an accepted public ID",
                ));
            }
            Ok(seen)
        })?;
    Ok(partition)
}

fn require_representative(group: &PublicIdentityGroup) -> ReportResult<()> {
    let Some(public_id) = group.public_id() else {
        return Ok(());
    };
    let representative = group
        .members
        .get(group.canonical_candidate())
        .ok_or_else(|| invalid("public identity candidate group has an absent canonical member"))?;
    let expected = successor(representative)
        .ok_or_else(|| invalid("public identity representative is not accepted"))?;
    if &expected.public_id != public_id
        || &expected.representative_candidate != group.canonical_candidate()
        || group
            .members
            .values()
            .any(|member| successor(member) != Some(expected))
    {
        return Err(invalid(
            "public identity representative differs from its retained successor",
        ));
    }
    Ok(())
}

fn successor(resolution: &PublicResolution) -> Option<&ResolvedSuccessor> {
    match resolution {
        PublicResolution::Canonical(identity)
        | PublicResolution::Redirect {
            successor: identity,
            ..
        }
        | PublicResolution::RevokedRedirect {
            successor: identity,
            ..
        } => Some(identity),
        PublicResolution::Unknown
        | PublicResolution::Unresolved
        | PublicResolution::Ambiguous { .. } => None,
    }
}
