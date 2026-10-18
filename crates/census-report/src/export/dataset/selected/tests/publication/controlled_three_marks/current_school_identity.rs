use super::source::{CurrentSchoolInput, InstalledSource, TestResult};
use census_domain::model::{
    AppliedIdentityKind, AthleteCandidateId, CanonicalAthlete, SourceNamespace,
    ATHLETE_IDENTITY_FAMILY,
};
use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_domain::public_identity_history::{HistoryRequest, PublicResolution};
use census_store::{SourceCandidateMembershipRequestV1, Table};

const OBSERVED_AT: &str = "2026-10-16";
const MEMBERSHIP_OPERATION: &str = "controlled-current-school-source-membership";

fn co_candidate(
    installed: &InstalledSource,
    input: &CurrentSchoolInput,
) -> TestResult<AthleteCandidateId> {
    let rows: Vec<CanonicalAthlete> = installed.store.snapshot().scan(Table::Athletes)?;
    let mut found: Option<AthleteCandidateId> = None;
    for row in &rows {
        if row.school != input.school {
            continue;
        }
        let mut linked = false;
        for identity in row.identities() {
            if identity.namespace == SourceNamespace::MilesplitAthlete && identity.id == "9999999" {
                linked = true;
                break;
            }
        }
        if !linked {
            continue;
        }
        if found.is_some() {
            return Err("controlled colorado candidate ambiguous".into());
        }
        found = Some(row.id.cast());
    }
    let candidate = found.ok_or("controlled colorado candidate missing")?;
    check!(candidate != installed.candidate);
    Ok(candidate)
}

fn resolved_case(
    installed: &InstalledSource,
    original: &AthleteCandidateId,
    current: &AthleteCandidateId,
) -> TestResult<ReviewCase> {
    let cases: Vec<ReviewCase> = installed.store.snapshot().scan(Table::ReviewCases)?;
    let mut selected: Option<ReviewCase> = None;
    for case in cases {
        if case.state != ReviewState::Resolved {
            continue;
        }
        if case.family != ATHLETE_IDENTITY_FAMILY {
            continue;
        }
        if case.member_ids.len() != 2 {
            continue;
        }
        if !case.member_ids.iter().any(|member| member == original) {
            continue;
        }
        if !case.member_ids.iter().any(|member| member == current) {
            continue;
        }
        if selected.is_some() {
            return Err("controlled identity case ambiguous".into());
        }
        selected = Some(case);
    }
    let case = selected.ok_or("controlled resolved identity case missing")?;
    require_verdict(installed, &case)?;
    Ok(case)
}

fn require_verdict(installed: &InstalledSource, case: &ReviewCase) -> TestResult {
    let verdicts: Vec<ReviewVerdictRecord> =
        installed.store.snapshot().scan(Table::IdentityVerdicts)?;
    let mut selected: Option<ReviewVerdictRecord> = None;
    for verdict in verdicts {
        if verdict.case_id != case.id {
            continue;
        }
        if selected.is_some() {
            return Err("controlled identity verdict ambiguous".into());
        }
        selected = Some(verdict);
    }
    let verdict = selected.ok_or("controlled identity verdict missing")?;
    check!(verdict.accepted);
    check!(eq; verdict.value, "same_person");
    check!(eq; verdict.member_ids, case.member_ids);
    check!(eq; verdict.reviewer, census_review::RULE_REVIEWER);
    check!(eq; verdict.observed_at, OBSERVED_AT);
    let index = installed.store.snapshot().athlete_identity_index()?;
    check!(index.supports_identity(AppliedIdentityKind::SamePerson, &case.member_ids));
    Ok(())
}

fn anchor_binding(installed: &InstalledSource) -> TestResult<String> {
    let accepted = installed
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("controlled accepted registry missing")?;
    for record in accepted.history_records() {
        if let HistoryRequest::Establish { binding, .. } = &record.request {
            if binding.candidate == installed.candidate {
                return Ok(binding.id.clone());
            }
        }
    }
    Err("controlled source anchor missing".into())
}

fn require_owner(installed: &InstalledSource) -> TestResult {
    let accepted = installed
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("controlled accepted registry missing")?;
    match accepted.resolve_candidate(&installed.candidate)? {
        PublicResolution::Canonical(row) => {
            check!(eq; row.public_id, installed.public);
            Ok(())
        }
        PublicResolution::Redirect { successor, .. } => {
            check!(eq; successor.public_id, installed.public);
            Ok(())
        }
        PublicResolution::RevokedRedirect { successor, .. } => {
            check!(eq; successor.public_id, installed.public);
            Ok(())
        }
        PublicResolution::Unknown
        | PublicResolution::Unresolved
        | PublicResolution::Ambiguous { .. } => {
            Err("controlled original without registry root".into())
        }
    }
}

fn require_merged(
    installed: &InstalledSource,
    original: &AthleteCandidateId,
    current: &AthleteCandidateId,
) -> TestResult {
    let accepted = installed
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("controlled merged registry missing")?;
    for candidate in [original, current] {
        match accepted.resolve_candidate(candidate)? {
            PublicResolution::Canonical(row) => {
                check!(eq; row.public_id, installed.public);
            }
            PublicResolution::Redirect { successor, .. } => {
                check!(eq; successor.public_id, installed.public);
            }
            PublicResolution::RevokedRedirect { successor, .. } => {
                check!(eq; successor.public_id, installed.public);
            }
            PublicResolution::Unknown
            | PublicResolution::Unresolved
            | PublicResolution::Ambiguous { .. } => {
                return Err("controlled merged candidate unresolved".into());
            }
        }
    }
    Ok(())
}

pub(super) fn accept(installed: &InstalledSource, input: &CurrentSchoolInput) -> TestResult {
    let current = co_candidate(installed, input)?;
    census_review::reconcile_athletes(&installed.store, OBSERVED_AT, false)?;
    let case = resolved_case(installed, &installed.candidate, &current)?;
    require_owner(installed)?;
    let anchor = anchor_binding(installed)?;
    let request = SourceCandidateMembershipRequestV1 {
        case_id: case.id.clone(),
        owner: installed.public.clone(),
        anchor_binding_id: anchor,
        candidate: current.clone(),
    };
    let head = installed.identity_writer.head(&installed.store)?;
    let facts = installed.store.snapshot().tables_digest(&[
        Table::Athletes,
        Table::SourceObservations,
        Table::Performances,
        Table::RelayResults,
    ])?;
    let attached = installed.identity_writer.extend_source_candidate(
        &installed.store,
        MEMBERSHIP_OPERATION,
        &request,
        installed.store.snapshot().revision(),
        &head,
    )?;
    check!(!attached.replayed());
    check!(eq; attached.public_id(), &installed.public);
    check!(attached.introduced_public_ids().is_empty());
    check!(eq; attached.head().sequence, head.sequence.checked_add(1).ok_or("controlled identity sequence")?);
    check!(eq; installed.store.snapshot().tables_digest(&[
        Table::Athletes,
        Table::SourceObservations,
        Table::Performances,
        Table::RelayResults,
    ])?, facts);
    require_merged(installed, &installed.candidate, &current)
}
