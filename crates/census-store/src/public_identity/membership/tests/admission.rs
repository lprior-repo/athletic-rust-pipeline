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
    let inputs = crate::public_identity::context::Inputs::capture(&store.snapshot())?;
    let accepted = crate::public_identity::authority::accept_reviewed(
        &inputs,
        &request.case_id,
        "2026-10-15",
    )?;
    let ledger = writer.verified(store)?;
    let digest = super::request::digest(writer.store_identity(), "physical-check", request)?;
    check!(super::request::authorized(
        request,
        &digest,
        &inputs,
        &accepted,
        ledger.history.projection()
    )
    .is_ok());
    for change in 0..4 {
        let mut altered = crate::public_identity::context::Inputs::capture(&store.snapshot())?;
        let row = altered
            .athletes
            .iter_mut()
            .find(|row| row.id.as_str() == request.candidate.as_str())
            .ok_or("candidate physical row")?;
        match change {
            0 => row.source.as_mut().ok_or("primary")?.id = "copied-foreign-provider-key".into(),
            1 => row.grad_year = census_domain::model::GradYear::new(2028).ok_or("year")?,
            2 => row.gender = census_domain::model::Gender::Boys,
            _ => row.school = SchoolId::mint("sch", &["tampered-school-with-old-physical-id"]),
        }
        check!(super::request::authorized(
            request,
            &digest,
            &altered,
            &accepted,
            ledger.history.projection()
        )
        .is_err());
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
    let held: crate::public_identity::witness::Witness = serde_json::from_slice(&raw)?;
    let request = SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: birth.public_id().clone(),
        anchor_binding_id: format!("source_binding_v1_{}", held.request_digest),
        candidate: second.id.cast(),
    };
    let state = retained_state(&store)?;
    check!(writer
        .extend_source_candidate(
            &store,
            "conflicting-cohort",
            &request,
            store.snapshot().revision(),
            birth.head()
        )
        .is_err());
    check!(eq; retained_state(&store)?, state);
    Ok(())
}
