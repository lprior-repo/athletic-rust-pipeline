use super::*;

#[test]
fn attachment_refuses_copied_primary_cohort_gender_and_physical_id() -> TestResult {
    let fixture = ready()?;
    let Fixture {
        store,
        writer,
        request,
        ..
    } = &fixture;
    let inventory = stage_inventory(store, writer, "physical-check", &request.owner)?;
    let snapshot = store.snapshot();
    let view = crate::public_identity::paged_generation::VerifiedRawGraph::verify(
        &snapshot,
        inventory.graph().capture(),
        inventory.graph().seal(),
    )?;
    let accepted = crate::public_identity::authority::graph::accept_reviewed(
        &view,
        &request.case_id,
        "2026-10-15",
    )?;
    let ledger = writer.verified(store)?;
    let projection = ledger.history.projection();
    let anchor = projection
        .binding_claim(&request.anchor_binding_id)
        .ok_or("anchor binding")?;
    let rows = crate::public_identity::authority::graph::member_rows(
        &view,
        &[anchor.candidate.clone(), request.candidate.clone()],
    )?;
    let digest = super::request::digest(writer.store_identity(), "physical-check", request)?;
    let authorized =
        super::request::authorized_rows(request, &digest, &rows, &accepted, projection)?;
    check!(matches!(authorized,
        census_domain::public_identity_history::HistoryRequest::BindCandidateMembership {
            owner, membership, evidence,
        } if owner == request.owner
            && membership.id == format!("source_membership_v1_{digest}")
            && membership.candidate == request.candidate
            && membership.anchor_binding_id == request.anchor_binding_id
            && membership.source == anchor.source
            && evidence == census_domain::public_identity_history::CandidateDecisionReference::from_accepted(
                &accepted, census_domain::public_identity_history::ReplayLimits::SMALL)?
    ));
    for change in 0..4 {
        let mut altered = rows.clone();
        let (_, row) = altered
            .iter_mut()
            .find(|(_, row)| row.id.as_str() == request.candidate.as_str())
            .ok_or("candidate physical row")?;
        match change {
            0 => row.source.as_mut().ok_or("primary")?.id = "copied-foreign-provider-key".into(),
            1 => row.grad_year = census_domain::model::GradYear::new(2028).ok_or("year")?,
            2 => row.gender = census_domain::model::Gender::Boys,
            _ => row.school = SchoolId::mint("sch", &["tampered-school-with-old-physical-id"]),
        }
        check!(matches!(
            super::request::authorized_rows(request, &digest, &altered, &accepted, projection),
            Err(StoreError::PublicIdentity(
                PublicIdentityError::NotSourceBound
            ))
        ));
    }
    Ok(())
}

#[test]
fn same_primary_never_overrides_genuine_conflicting_cohort_review() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("membership-cohort", "7403")?;
    store.append(Table::Athletes, &first)?;
    let birth = writer.establish_source_bound(
        &store,
        "cohort-birth",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let mut second = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["conflicting-cohort-school"]),
        &first.canonical_name,
        census_domain::model::GradYear::new(2028).ok_or("cohort")?,
        first.gender,
        first.source.clone().ok_or("primary")?,
    )?;
    second.evidence = first.evidence.clone();
    store.append(Table::Athletes, &second)?;
    let (case, verdict) = review(&store, &[first.id.cast(), second.id.cast()], "same_person")?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let raw = store
        .journal
        .get(crate::public_identity::witness::key(1))?
        .ok_or("birth witness")?;
    let held = HeldGraphWitness::decode("birth witness", &raw)?;
    check!(eq; held.version, 8);
    let request = SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: birth.public_id().clone(),
        anchor_binding_id: format!("source_binding_v1_{}", held.request_digest),
        candidate: second.id.cast(),
    };
    stage_inventory(&store, &writer, "conflicting-cohort", birth.public_id())?;
    let state = retained_state(&store)?;
    check!(matches!(
        writer.extend_source_candidate(
            &store,
            "conflicting-cohort",
            &request,
            store.snapshot().revision(),
            birth.head()
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::NotReviewed))
    ));
    check!(eq; retained_state(&store)?, state);
    Ok(())
}
