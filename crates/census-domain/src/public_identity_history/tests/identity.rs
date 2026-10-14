use super::fixture::*;
use super::*;

#[test]
fn birth_uses_the_literal_installation_ordinal_id() -> TestResult {
    let mut fixture = Fixture::new()?;
    let born = fixture.append(establish("z", "1"))?;
    check!(eq; born, vec![PublicAthleteId::parse(FIRST)?]);
    check!(eq; fixture.project()?.resolve_public(&PublicAthleteId::parse(FIRST)?)?,
        PublicResolution::Canonical(successor(FIRST, "z", 1, 0)?));
    Ok(())
}

#[test]
fn later_smaller_candidate_does_not_replace_the_first_public_id() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("z", "1"))?;
    let _second = fixture.append(establish("a", "2"))?;
    check!(candidate("a") < candidate("z"));
    fixture.merge(FIRST, &[SECOND, FIRST], &["a", "z"], "merge:1")?;
    let projection = fixture.project()?;
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(FIRST)?)?,
        PublicResolution::Canonical(successor(FIRST, "z", 1, 0)?));
    check!(eq; projection.resolve_public(&PublicAthleteId::parse(SECOND)?)?,
        PublicResolution::Redirect { successor: successor(FIRST, "z", 1, 0)?, changed_at: 3 });
    Ok(())
}

#[test]
fn nonoldest_merge_target_is_refused() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("z", "1"))?;
    let _second = fixture.append(establish("a", "2"))?;
    fixture.merge(SECOND, &[FIRST, SECOND], &["a", "z"], "merge:1")?;
    check!(eq; fixture.project().err(), Some(HistoryError::NonCanonicalTarget));
    Ok(())
}

#[test]
fn redirected_member_is_not_a_new_merge_target() -> TestResult {
    let mut fixture = chain()?;
    fixture.merge(FIRST, &[FIRST, SECOND], &["a", "b"], "merge:again")?;
    check!(eq; fixture.project().err(), Some(HistoryError::NonCanonicalMember));
    Ok(())
}

#[test]
fn unknown_public_id_is_explicit() -> TestResult {
    let fixture = Fixture::new()?;
    check!(eq; fixture.project()?.resolve_public(&PublicAthleteId::parse(FIRST)?)?, PublicResolution::Unknown);
    check!(eq; fixture.project()?.resolve_binding("missing")?, BindingResolution::Unknown);
    check!(eq; fixture.project()?.resolve_candidate(&candidate("missing"))?, PublicResolution::Unknown);
    Ok(())
}

#[test]
fn normalized_member_order_produces_identical_candidate_records() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let _second = fixture.append(establish("b", "2"))?;
    let first = HistoryRequest::Merge {
        canonical: PublicAthleteId::parse(FIRST)?,
        members: vec![
            PublicAthleteId::parse(FIRST)?,
            PublicAthleteId::parse(SECOND)?,
        ],
        evidence: evidence("merge:ab", CandidateEvidenceKind::SamePerson, &["a", "b"]),
    };
    let second = HistoryRequest::Merge {
        canonical: PublicAthleteId::parse(FIRST)?,
        members: vec![
            PublicAthleteId::parse(SECOND)?,
            PublicAthleteId::parse(FIRST)?,
        ],
        evidence: evidence("merge:ab", CandidateEvidenceKind::SamePerson, &["b", "a"]),
    };
    check!(eq; prepare_next(&fixture.installation, &fixture.head, first, ReplayLimits::SMALL)?,
        prepare_next(&fixture.installation, &fixture.head, second, ReplayLimits::SMALL)?);
    Ok(())
}

#[test]
fn rebuilding_the_same_history_keeps_literal_alias_resolution() -> TestResult {
    let fixture = chain()?;
    for projection in [fixture.project()?, fixture.project()?] {
        check!(eq; projection.resolve_public(&PublicAthleteId::parse(THIRD)?)?,
            PublicResolution::Redirect { successor: successor(FIRST, "a", 1, 0)?, changed_at: 5 });
    }
    Ok(())
}
