use super::*;

#[path = "identity_aliases_tests/cen14_captures.rs"]
mod cen14_captures;

fn candidate(name: &str) -> AthleteCandidateId {
    AthleteCandidateId::mint("can", &[name])
}

#[test]
fn joining_in_either_order_roots_at_the_smaller_candidate() -> Result<(), Box<dyn std::error::Error>>
{
    let left = candidate("alpha");
    let right = candidate("beta");
    let smaller = left.clone().min(right.clone());
    let mut forward = IdentityAliases::default();
    forward.join(&left, &right)?;
    let mut backward = IdentityAliases::default();
    backward.join(&right, &left)?;
    check!(eq; forward.root(&left)?.as_str(), smaller.as_str());
    check!(eq; backward.root(&left)?.as_str(), smaller.as_str());
    check!(eq; forward.flattened()?, backward.flattened()?);
    Ok(())
}

#[test]
fn a_smaller_candidate_joined_last_becomes_the_single_root(
) -> Result<(), Box<dyn std::error::Error>> {
    let first = candidate("aa");
    let second = candidate("bb");
    let third = candidate("cc");
    let mut smaller = first.clone();
    for other in [&second, &third] {
        smaller = smaller.min(other.clone());
    }
    let mut aliases = IdentityAliases::default();
    aliases.join(&second, &third)?;
    aliases.join(&first, &third)?;
    for member in [&first, &second, &third] {
        check!(eq; aliases.root(member)?.as_str(), smaller.as_str());
    }
    Ok(())
}

#[test]
fn every_join_preserves_the_strictly_decreasing_parent_invariant(
) -> Result<(), Box<dyn std::error::Error>> {
    let members = [
        candidate("p"),
        candidate("q"),
        candidate("r"),
        candidate("s"),
    ];
    let mut aliases = IdentityAliases::default();
    for (index, left) in members.iter().enumerate() {
        for right in members.iter().skip(index.saturating_add(1)) {
            aliases.join(left, right)?;
        }
    }
    let expected = members
        .iter()
        .min()
        .cloned()
        .ok_or(IdentityError::UnknownSubject("empty fixture".to_owned()))?;
    for member in &members {
        check!(eq; aliases.root(member)?.as_str(), expected.as_str());
    }
    for (child, parent) in &aliases.flattened()? {
        check!(parent < child);
    }
    Ok(())
}

#[test]
fn a_corrupt_parent_chain_is_refused_instead_of_merging() {
    let smaller = candidate("x");
    let larger = candidate("y");
    let (smaller, larger) = if smaller < larger {
        (smaller, larger)
    } else {
        (larger, smaller)
    };
    let mut aliases = IdentityAliases::default();
    aliases.parents.insert(smaller.clone(), larger.clone());
    assert!(matches!(
        aliases.root(&smaller),
        Err(IdentityError::AliasCycle)
    ));
    assert!(matches!(
        aliases.flattened(),
        Err(IdentityError::AliasCycle)
    ));
}

fn component_member(
    school: &str,
    source: super::super::SourceIdentity,
) -> super::super::CanonicalAthlete {
    use super::super::*;
    let school = SchoolId::mint("sch", &["WI", school]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some(format!("https://fixture.test/{school}"))),
        "2026-10-07",
    ));
    athlete
}

fn component_review(
    index: &super::super::AthleteIdentityIndex,
    ids: &[AthleteCandidateId],
) -> Result<(super::super::ReviewCase, super::super::ReviewVerdictRecord), Box<dyn std::error::Error>>
{
    use super::super::*;
    let subject = "Transitive provider identity";
    let detail = "Independently attested shared provider object";
    let evidence = index.case_evidence(subject, detail, ids)?;
    let first = ids.first().ok_or("missing component member")?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = ids.to_vec();
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
        rationale: detail.into(),
        reviewer: "deterministic fixture".into(),
        observed_at: "2026-10-07".into(),
        member_ids: case.member_ids.clone(),
    };
    Ok((case, verdict))
}

fn component_projection(
    members: &[super::super::CanonicalAthlete; 3],
    reverse: bool,
) -> Result<super::super::AthleteIdentityProjection, Box<dyn std::error::Error>> {
    use super::super::*;
    let mut index = AthleteIdentityIndex::default();
    for member in members {
        index.observe(member)?;
    }
    let [first, middle, last] = members;
    let (ab, ab_verdict) = component_review(&index, &[first.id.cast(), middle.id.cast()])?;
    let (bc, bc_verdict) = component_review(&index, &[middle.id.cast(), last.id.cast()])?;
    let cases = [ab, bc];
    let verdicts = [ab_verdict, bc_verdict];
    let mut builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let mut decisions = builder
        .reviewed_applications("2026-10-07")
        .map(|(_, application)| match application? {
            IdentityApplication::Accepted(decision) => Ok(decision),
            IdentityApplication::Retained(issue) => {
                Err(format!("unexpected pair rejection: {issue:?}").into())
            }
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    if reverse {
        decisions.reverse();
    }
    for decision in decisions {
        check!(eq; builder.consider(decision.record())?, None);
    }
    Ok(builder.finish()?)
}

fn component_chain(
    last: super::super::SourceIdentity,
) -> Result<[super::super::CanonicalAthlete; 3], Box<dyn std::error::Error>> {
    use super::super::*;
    let mut first = component_member(
        "School A",
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1")
            .with_url("https://milesplit.test/1"),
    );
    first.add_identity(
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "9")
            .with_url("https://milesplit.test/1"),
    );
    let mut middle = component_member(
        "School B",
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "9").with_url("https://tfrrs.test/9"),
    );
    let mut last = component_member("School C", last);
    last.add_identity(
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "9")
            .with_url("https://fixture.test/last"),
    );
    for (member, publisher) in [&mut first, &mut middle, &mut last].into_iter().zip([
        "first-fixture",
        "middle-fixture",
        "last-fixture",
    ]) {
        member
            .identity_attestations
            .push(cen14_captures::claim(publisher)?);
    }
    Ok([first, middle, last])
}

#[test]
fn transitive_disjoint_provider_objects_retain_every_member_without_aliases_in_both_orders(
) -> Result<(), Box<dyn std::error::Error>> {
    use super::super::*;
    let members = component_chain(
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "2")
            .with_url("https://milesplit.test/2"),
    )?;
    for reverse in [false, true] {
        let projection = component_projection(&members, reverse)?;
        for member in &members {
            check!(eq; projection.status(member.id.as_str())?, IdentityStatus::RetainedConflict);
            check!(eq; projection.canonical_id(member.id.as_str()), member.id.as_str());
        }
        check!(eq; projection.rejected_applications().get(
            &IdentityDecisionIssue::ConflictingApplications
        ), Some(&1));
    }
    Ok(())
}

#[test]
fn transitive_compatible_provider_chain_verifies_every_member_in_both_orders(
) -> Result<(), Box<dyn std::error::Error>> {
    use super::super::*;
    let members = component_chain(
        SourceIdentity::new(SourceNamespace::DirectAthleticsAthlete, "2")
            .with_url("https://directathletics.test/2"),
    )?;
    let canonical = members
        .iter()
        .map(|member| member.id.as_str())
        .min()
        .ok_or("empty component")?;
    for reverse in [false, true] {
        let projection = component_projection(&members, reverse)?;
        for member in &members {
            check!(eq; projection.status(member.id.as_str())?, IdentityStatus::Verified);
            check!(eq; projection.canonical_id(member.id.as_str()), canonical);
        }
        check!(eq; projection.rejected_applications().len(), 0);
    }
    Ok(())
}
