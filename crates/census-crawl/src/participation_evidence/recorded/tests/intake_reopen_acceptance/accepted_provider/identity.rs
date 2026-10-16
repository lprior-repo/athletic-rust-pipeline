use super::*;
use census_domain::model::{athlete_identity_digest, ATHLETE_IDENTITY_POLICY};

pub(super) fn verify(
    store: &Store,
    installed: &installed::Installed,
    outcome: &IdentityCommitOutcomeV1,
) -> TestResult {
    installed.verify(store)?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted source ledger")?;
    check!(eq; accepted.head(), outcome.head());
    check!(eq; accepted.public_ids().cloned().collect::<Vec<_>>(), vec![outcome.public_id().clone()]);
    let resolution = PublicResolution::Canonical(ResolvedSuccessor {
        public_id: outcome.public_id().clone(),
        representative_candidate: installed.candidate.clone(),
        established: EstablishmentOrdinal {
            sequence: 1,
            position: 0,
        },
    });
    check!(eq; accepted.resolve_candidate(&installed.candidate)?, resolution);
    check!(eq; accepted.resolve_public(outcome.public_id())?, resolution);
    let [record] = accepted.history_records() else {
        return Err("exactly one source establishment history record required".into());
    };
    check!(eq; record.installation, snapshot.store_identity()?);
    check!(eq; record.sequence, 1);
    check!(eq; record.previous.sequence, 0);
    let HistoryRequest::Establish { binding, evidence } = &record.request else {
        return Err("genuine source must establish through source binding".into());
    };
    check!(eq; binding.candidate, installed.candidate);
    check!(eq; binding.source.namespace, SourceNamespace::MilesplitAthlete);
    check!(eq; binding.source.identifier, "7223777");
    check!(eq; binding.effective, EffectiveScope::Unknown);
    check!(eq; binding.supersedes, None);
    check!(eq; evidence.kind, CandidateEvidenceKind::SourceBound);
    check!(eq; evidence.members.as_slice(), std::slice::from_ref(&installed.candidate));
    check!(eq; accepted.resolve_binding(&binding.id)?, BindingResolution::Resolved {
        identity: resolution, established_at: 1, superseded_by: None,
    });
    let decisions = snapshot.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let [decision] = decisions.as_slice() else {
        return Err("exactly one genuine source acceptance decision required".into());
    };
    check!(eq; decision.policy, ATHLETE_IDENTITY_POLICY);
    check!(eq; decision.kind, AppliedIdentityKind::SourceBound);
    check!(eq; decision.canonical_id.as_ref(), Some(&installed.athlete.id));
    check!(eq; decision.case_id, None);
    check!(eq; decision.verdict_digest, None);
    let [member] = decision.members.as_slice() else {
        return Err("source-bound decision must contain the installed candidate only".into());
    };
    check!(eq; member.subject, installed.candidate);
    check!(eq; member.evidence_digest, athlete_identity_digest(&installed.athlete)?);
    check!(eq; evidence.application_id, decision.id);
    check!(evidence.matches_applied(decision, ReplayLimits::SMALL)?);
    Ok(())
}
