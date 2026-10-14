use super::{fixture::*, TestResult};
use crate::affiliation_candidates::*;
use crate::model::{ProfileDateRangeV1, SchoolYear, Sport};

#[test]
fn explicit_interval_is_inclusive_without_automatic_extension() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claims = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &owners,
        key("claim-A")?,
    )?)?];
    for value in ["2026-09-01", "2027-06-15"] {
        let as_of = day(value)?;
        let mut query = fixture.query()?;
        query.as_of = &as_of;
        assert_eq!(
            select_candidates(query, &claims, &[])?.outcome,
            CandidateOutcome::SingleCandidate(claims.first().ok_or("synthetic claim absent")?)
        );
    }
    for value in ["2026-08-31"] {
        let as_of = day(value)?;
        let mut query = fixture.query()?;
        query.as_of = &as_of;
        assert!(matches!(
            select_candidates(query, &claims, &[])?.outcome,
            CandidateOutcome::Unknown
        ));
    }
    let after = day("2027-06-16")?;
    let mut query = fixture.query()?;
    query.as_of = &after;
    assert!(matches!(
        select_candidates(query, &claims, &[])?.outcome,
        CandidateOutcome::EndedWithoutReplacement
    ));
    assert_eq!(
        ProfileDateRangeV1::new(day("2027-06-15")?, day("2026-09-01")?),
        Err(crate::model::ProfileError::ReversedDateRange)
    );
    Ok(())
}

#[test]
fn exact_day_does_not_carry_forward() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let exact = day("2026-10-09")?;
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.temporal = TemporalScope::ExactDay(&exact);
    let claims = [LocallyCheckedAssertion::check(raw)?];
    let claim = claims.first().ok_or("synthetic claim absent")?;
    for date in ["2026-10-08", "2026-10-09", "2026-10-10"] {
        let as_of = day(date)?;
        let mut query = fixture.query()?;
        query.as_of = &as_of;
        let selected = select_candidates(query, &claims, &[])?;
        let (outcome, disposition) = if date == "2026-10-09" {
            (
                CandidateOutcome::SingleCandidate(claim),
                Disposition::Candidate,
            )
        } else {
            (
                CandidateOutcome::Unknown,
                Disposition::OutsideEffectiveScope,
            )
        };
        assert_eq!(selected.outcome, outcome);
        assert_eq!(
            selected.facts,
            [FactDisposition::Assertion {
                fact: claim,
                disposition
            }]
        );
    }
    Ok(())
}

#[test]
fn undated_and_ongoing_contract_gaps_cannot_select() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    for gap in [
        TemporalGap::Undated,
        TemporalGap::AmbiguousLabel,
        TemporalGap::UnknownBoundary,
        TemporalGap::MissingSeasonCalendar,
    ] {
        raw.temporal = TemporalScope::Unresolved(gap);
        let claims = [LocallyCheckedAssertion::check(raw)?];
        assert!(matches!(
            select_candidates(fixture.query()?, &claims, &[])?.outcome,
            CandidateOutcome::Unknown
        ));
    }
    raw.temporal = TemporalScope::ExplicitOngoingFrom {
        start: fixture.interval.from(),
        continuing_tenure: SourceSpan::new(0, BODY.len())?,
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    assert!(matches!(
        select_candidates(fixture.query()?, &claims, &[])?
            .facts
            .first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::TemporalUnresolved(TemporalGap::OngoingContractUnresolved),
            ..
        })
    ));
    Ok(())
}

#[test]
fn xc_roster_never_becomes_track_enrollment() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.relationship = RelationshipCandidate::RepresentsSchoolProgram {
        school: &fixture.school,
        program: &fixture.program,
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    let xc = select_candidates(fixture.query()?, &claims, &[])?;
    assert!(matches!(xc.outcome, CandidateOutcome::SingleCandidate(fact)
        if matches!(fact.raw().relationship, RelationshipCandidate::RepresentsSchoolProgram { .. })));
    let mut track = fixture.query()?;
    track.sport = Sport::OutdoorTrack;
    assert!(matches!(
        select_candidates(track, &claims, &[])?.outcome,
        CandidateOutcome::Unknown
    ));
    Ok(())
}

#[test]
fn coop_program_without_athlete_campus_binding_stays_unknown() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.owner.target = OwnerTarget::Program(&fixture.program);
    raw.relationship = RelationshipCandidate::RepresentsCoopProgram {
        program: &fixture.program,
        member: CoopMemberCandidate::Unknown,
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    let result = select_candidates(fixture.query()?, &claims, &[])?;
    assert!(matches!(result.outcome, CandidateOutcome::Unknown));
    assert!(matches!(
        result.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::CoopMemberUnknown,
            ..
        })
    ));
    Ok(())
}

#[test]
fn season_uses_supplied_calendar_and_keeps_year_and_sport_binding() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let interval = ProfileDateRangeV1::new(day("2026-09-17")?, day("2026-11-03")?)?;
    let year = SchoolYear::new(2026).ok_or("synthetic school year")?;
    let calendar = CalendarCandidate {
        year,
        sport: Sport::CrossCountry,
        interval: &interval,
        raw_label: b"fall 2026",
        label_convention: key("synthetic-fall-label")?,
        mapping_rule: &fixture.rule,
        evidence: &evidence,
        locator: key("synthetic-calendar-row")?,
        statement: SourceSpan::new(0, BODY.len())?,
        known: known(1, 3)?,
    };
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.temporal = TemporalScope::AcademicSeason {
        year,
        sport: Sport::CrossCountry,
        calendar: &calendar,
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    let claim = claims.first().ok_or("synthetic claim absent")?;
    for date in [
        "2026-09-16",
        "2026-09-17",
        "2026-10-10",
        "2026-11-03",
        "2026-11-04",
    ] {
        let as_of = day(date)?;
        let mut query = fixture.query()?;
        query.as_of = &as_of;
        let selected = select_candidates(query, &claims, &[])?;
        let (outcome, disposition) = match date {
            "2026-09-16" => (
                CandidateOutcome::Unknown,
                Disposition::OutsideEffectiveScope,
            ),
            "2026-11-04" => (
                CandidateOutcome::EndedWithoutReplacement,
                Disposition::ExplicitIntervalEnded,
            ),
            _ => (
                CandidateOutcome::SingleCandidate(claim),
                Disposition::Candidate,
            ),
        };
        assert_eq!(selected.outcome, outcome);
        assert_eq!(
            selected.facts,
            [FactDisposition::Assertion {
                fact: claim,
                disposition
            }]
        );
    }
    raw.temporal = TemporalScope::AcademicSeason {
        year,
        sport: Sport::OutdoorTrack,
        calendar: &calendar,
    };
    assert_eq!(
        LocallyCheckedAssertion::check(raw),
        Err(AffiliationCandidateError::CalendarScopeMismatch)
    );
    Ok(())
}

#[test]
fn coop_campus_requires_both_effective_intervals_and_exact_source_person() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let expired = ProfileDateRangeV1::new(day("2026-09-01")?, day("2026-09-30")?)?;
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.owner.target = OwnerTarget::Program(&fixture.program);
    raw.relationship = RelationshipCandidate::RepresentsCoopProgram {
        program: &fixture.program,
        member: CoopMemberCandidate::Claimed {
            school: &fixture.school,
            athlete_member_binding: key("synthetic-campus-binding")?,
            source_person: &fixture.person,
            campus_binding_interval: &fixture.interval,
            program_membership_interval: &expired,
            membership_statement: SourceSpan::new(0, BODY.len())?,
            known: known(1, 3)?,
        },
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    assert!(matches!(
        select_candidates(fixture.query()?, &claims, &[])?.outcome,
        CandidateOutcome::Unknown
    ));
    raw.relationship = RelationshipCandidate::RepresentsCoopProgram {
        program: &fixture.program,
        member: CoopMemberCandidate::Claimed {
            school: &fixture.school,
            athlete_member_binding: key("synthetic-campus-binding")?,
            source_person: &fixture.person,
            campus_binding_interval: &fixture.interval,
            program_membership_interval: &fixture.interval,
            membership_statement: SourceSpan::new(0, BODY.len())?,
            known: known(1, 3)?,
        },
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    assert_eq!(
        select_candidates(fixture.query()?, &claims, &[])?.outcome,
        CandidateOutcome::SingleCandidate(claims.first().ok_or("synthetic claim absent")?)
    );
    if let RelationshipCandidate::RepresentsCoopProgram {
        program,
        member:
            CoopMemberCandidate::Claimed {
                school,
                athlete_member_binding,
                campus_binding_interval,
                program_membership_interval,
                membership_statement,
                known,
                ..
            },
    } = raw.relationship
    {
        raw.relationship = RelationshipCandidate::RepresentsCoopProgram {
            program,
            member: CoopMemberCandidate::Claimed {
                school,
                athlete_member_binding,
                source_person: &fixture.other_owner,
                campus_binding_interval,
                program_membership_interval,
                membership_statement,
                known,
            },
        };
    }
    assert_eq!(
        LocallyCheckedAssertion::check(raw),
        Err(AffiliationCandidateError::CoopPersonMismatch)
    );
    Ok(())
}
