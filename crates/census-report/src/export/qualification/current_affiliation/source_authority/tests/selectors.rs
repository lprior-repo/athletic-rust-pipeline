use super::*;
use fixture::{install, Fixture};

fn unselected(
    fixture: &Fixture,
    verified: &crate::export::VerifiedCurrentAffiliations,
    athlete: &AthleteId,
) -> TestResult {
    let dataset = fixture.dataset()?;
    let selected = crate::report::Derivation::selected(
        &dataset,
        crate::report::Scope::AllSources,
        None,
        &[census_domain::UsJurisdiction::Michigan],
        verified,
    )?;
    check!(selected.athletes().iter().all(|row| &row.id != athlete));
    check!(eq; selected.affiliation_exclusions().get(athlete), Some(&crate::report::CurrentAffiliationExclusion::Unresolved));
    Ok(())
}

fn attach(
    fixture: &Fixture,
    anchor: &AthleteId,
    candidate: &AthleteId,
    operation: &str,
) -> TestResult {
    let accepted = fixture
        .store
        .snapshot()
        .freeze_public_identity_v2()?
        .replay()?;
    let census_domain::public_identity_history::PublicResolution::Canonical(owner) =
        accepted.resolve_candidate(&anchor.cast())?
    else {
        return Err("controlled accepted anchor".into());
    };
    let binding = accepted
        .history_records()
        .iter()
        .find_map(|record| match &record.request {
            census_domain::public_identity_history::HistoryRequest::Establish {
                binding, ..
            } if binding.candidate == anchor.cast() => Some(binding.id.clone()),
            _ => None,
        })
        .ok_or("controlled anchor binding")?;
    let mut index = AthleteIdentityIndex::default();
    for athlete in fixture.store.scan::<CanonicalAthlete>(Table::Athletes)? {
        index.observe(&athlete)?;
    }
    let members = vec![anchor.cast(), candidate.cast()];
    let evidence = index.case_evidence(
        "Controlled selector membership",
        "Controlled same-provider-person fixture",
        &members,
    )?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        anchor.as_str(),
        "Controlled selector membership",
        "Controlled same-provider-person fixture",
        evidence,
    );
    case.member_ids = members;
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: "Controlled fixture only".into(),
        reviewer: "controlled regression fixture".into(),
        observed_at: fixture::AT.into(),
        member_ids: case.member_ids.clone(),
    };
    fixture
        .store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &[verdict])?;
    fixture.writer.extend_source_candidate(
        &fixture.store,
        operation,
        &census_store::SourceCandidateMembershipRequestV1 {
            case_id: case.id,
            owner: owner.public_id,
            anchor_binding_id: binding,
            candidate: candidate.cast(),
        },
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    Ok(())
}

mod authority;

mod gaps;

mod identity;

mod physical;
