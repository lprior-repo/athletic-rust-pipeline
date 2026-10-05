use super::*;

#[test]
fn a_mailbox_bound_to_the_current_role_program_claim_publishes() -> TestResult {
    let mut coach = head("Bound coach", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("  bound@example.invalid  ".into());
    bind(&mut coach);
    let binding = coach.tenure_evidence[0]
        .claim
        .as_mut()
        .ok_or("missing binding")?;
    binding.mailbox = Some("bound@example.invalid".into());
    let result = selected(&[coach], &athlete())?;
    check!(eq; result.state, ContactState::ProfessionalCoachEmail);
    check!(eq; result.name, "Bound coach");
    check!(eq; result.email, "bound@example.invalid");
    Ok(())
}

#[test]
fn an_unbound_mailbox_with_current_tenure_resolves_unknown() -> TestResult {
    let mut coach = head("Unbound coach", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("unbound@example.invalid".into());
    coach.tenure_evidence[0].claim = None;
    let result = selected(&[coach], &athlete())?;
    check!(eq; result.state, ContactState::ContactResearchUnknown);
    check!(eq; result.name, "");
    check!(eq; result.email, "");
    Ok(())
}

#[test]
fn a_name_only_claim_never_binds_a_present_mailbox() -> TestResult {
    let mut coach = head("Name only", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("unbound@example.invalid".into());
    let result = selected(&[coach], &athlete())?;
    check!(eq; result.state, ContactState::ContactResearchUnknown);
    check!(eq; result.email, "");
    Ok(())
}

#[test]
fn each_person_school_role_and_program_component_must_match() -> TestResult {
    for mismatch in 0..5 {
        let mut coach = head("Bound coach", Sport::OutdoorTrack, Gender::Boys)?;
        coach.professional_email = Some("bound@example.invalid".into());
        bind(&mut coach);
        let binding = coach.tenure_evidence[0]
            .claim
            .as_mut()
            .ok_or("missing binding")?;
        match mismatch {
            0 => binding.coach = census_domain::model::CoachId::mint("coa", &["other person"]),
            1 => binding.school = SchoolId::mint("sch", &["other school"]),
            2 => binding.role = CoachRole::AssistantCoach,
            3 => {
                binding.program = CoachContactProgram::Team {
                    sport: Sport::CrossCountry,
                    gender: Gender::Boys,
                };
            }
            _ => {
                binding.program = CoachContactProgram::Team {
                    sport: Sport::OutdoorTrack,
                    gender: Gender::Girls,
                };
            }
        }
        let result = selected(&[coach], &athlete())?;
        check!(eq; result.state, ContactState::ContactResearchUnknown);
        check!(eq; result.email, "");
    }
    Ok(())
}

#[test]
fn professional_and_personal_mailboxes_require_independent_bindings() -> TestResult {
    for bind_professional in [true, false] {
        let mut coach = head("Two addresses", Sport::OutdoorTrack, Gender::Boys)?;
        coach.professional_email = Some("work@example.invalid".into());
        coach.personal_email = Some("personal@outlook.com".into());
        bind(&mut coach);
        let expected = if bind_professional {
            "work@example.invalid"
        } else {
            "personal@outlook.com"
        };
        coach.tenure_evidence.retain(|fact| {
            fact.claim
                .as_ref()
                .is_some_and(|binding| binding.mailbox.as_deref() == Some(expected))
        });
        let contacts = contacts(&[coach], year()?);
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        check!(eq; scope.all_emails(), expected);
        check!(eq; scope.professional_coach_email(),
        bind_professional.then_some("work@example.invalid"));
        check!(eq; scope.preferred().state, if bind_professional {
            ContactState::ProfessionalCoachEmail
        } else {
            ContactState::PersonalCoachEmail
        });
    }
    Ok(())
}

#[test]
fn a_name_only_current_fact_cannot_borrow_a_mailbox_from_another_season() -> TestResult {
    let mut coach = head("Two seasons", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("old@example.invalid".into());
    let name_only = coach.tenure_evidence[0].clone();
    bind(&mut coach);
    coach.tenure_evidence[0].tenure = CoachTenure::Current {
        school_year: SchoolYear::new(2025).ok_or("invalid season")?,
    };
    coach.tenure_evidence.push(name_only);
    let result = selected(&[coach], &athlete())?;
    check!(eq; result.state, ContactState::ContactResearchUnknown);
    check!(eq; result.email, "");
    Ok(())
}

#[test]
fn a_school_athletics_director_mailbox_publishes_without_a_team() -> TestResult {
    let mut coach = director()?;
    coach.professional_email = Some("director@example.invalid".into());
    bind(&mut coach);
    let result = selected(&[coach], &athlete())?;
    check!(eq; result.state, ContactState::ProfessionalAdEmail);
    check!(eq; result.email, "director@example.invalid");
    Ok(())
}

#[test]
fn a_different_mailbox_in_the_same_role_claim_does_not_publish() -> TestResult {
    let mut coach = head("Different address", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("selected@example.invalid".into());
    bind(&mut coach);
    coach.tenure_evidence[0]
        .claim
        .as_mut()
        .ok_or("missing binding")?
        .mailbox = Some("other@example.invalid".into());
    let result = selected(&[coach], &athlete())?;
    check!(eq; result.state, ContactState::ContactResearchUnknown);
    check!(eq; result.email, "");
    Ok(())
}

#[test]
fn a_current_unbound_observation_cannot_replace_a_bound_mailbox() -> TestResult {
    let mut bound = head("Same coach", Sport::OutdoorTrack, Gender::Boys)?;
    bound.professional_email = Some("bound@example.invalid".into());
    bind(&mut bound);
    let mut unbound = bound.clone();
    unbound.professional_email = Some("unbound@example.invalid".into());
    unbound.tenure_evidence[0].claim = None;
    for rows in [vec![bound.clone(), unbound.clone()], vec![unbound, bound]] {
        let result = selected(&rows, &athlete())?;
        check!(eq; result.state, ContactState::ProfessionalCoachEmail);
        check!(eq; result.email, "bound@example.invalid");
    }
    Ok(())
}
