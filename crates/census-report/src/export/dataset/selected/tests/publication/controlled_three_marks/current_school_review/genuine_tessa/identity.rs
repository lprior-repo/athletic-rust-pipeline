use super::*;
use census_domain::model::{AppliedAthleteIdentity, AppliedIdentityKind, AthleteCandidateId};
use census_domain::public_identity_history::PublicResolution;

pub(super) fn separate(
    store: &Store,
    target: &CanonicalAthlete,
    native_candidate: &AthleteCandidateId,
    public: &PublicAthleteId,
) -> TestResult {
    check!(ne; target.id.as_str(), native_candidate.as_str());
    let target_source = target.source.as_ref().ok_or("actual MS target primary")?;
    check!(eq; target_source.namespace,SourceNamespace::MilesplitAthlete);
    check!(eq; target_source.id,"13658157");
    let snapshot = store.snapshot();
    check!(eq; snapshot.get_merged::<CanonicalAthlete>(Table::Athletes,target.id.as_str())?.as_ref(),Some(target));
    check!(snapshot
        .get_merged::<CanonicalAthlete>(Table::Athletes, native_candidate.as_str())?
        .is_none());
    let athletes = snapshot.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(!athletes.iter().any(|athlete| {
        athlete.identities().any(|source| {
            source.namespace == SourceNamespace::TfrrsAthlete && source.id == "8779607"
        })
    }));
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("actual established MS public target")?;
    check!(
        matches!(accepted.resolve_candidate(&target.id.cast())?,PublicResolution::Canonical(row) if &row.public_id == public)
    );
    check!(matches!(
        accepted.resolve_candidate(native_candidate)?,
        PublicResolution::Unknown
    ));
    let bindings = accepted.component_bindings(public)?;
    check!(eq; bindings.len(), 1);
    for id in bindings {
        let binding = accepted
            .binding_claim(&id)
            .ok_or("actual MS source binding")?;
        check!(eq; binding.candidate.as_str(), target.id.as_str());
        check!(eq; binding.source.namespace, SourceNamespace::MilesplitAthlete);
        check!(eq; binding.source.identifier, "13658157");
    }
    let decisions = snapshot.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let [decision] = decisions.as_slice() else {
        return Err("only the actual MS source birth decision is expected".into());
    };
    check!(eq; decision.kind, AppliedIdentityKind::SourceBound);
    check!(
        matches!(decision.members.as_slice(), [member] if member.subject.as_str() == target.id.as_str())
    );
    let members = [target.id.cast(), native_candidate.clone()];
    let verdicts =
        snapshot.scan::<census_domain::model::ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    check!(!verdicts.iter().any(|verdict| verdict
        .member_ids
        .iter()
        .any(|candidate| members.contains(candidate))));
    Ok(())
}

mod attachment;
pub(super) use attachment::{linked, unattached};
