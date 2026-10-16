use super::fixture::{day, range};
use super::*;
use crate::model::{GradYear, QueryProfileV1, SchoolYear};
use sha2::{Digest, Sha256};

fn fixture() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    let mut input = fixture.profile.input().clone();
    input.policies.participation = participation_policy_v4()?;
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    Ok(fixture)
}

fn fact<'a>(
    fixture: &'a Fixture,
    interval: &'a crate::model::ProfileDateRangeV1,
) -> TestResult<InSeasonWindowFact<'a>> {
    Ok(InSeasonWindowFact {
        identity: EligibilityWindowIdentity {
            school_year: SchoolYear::new(2026).ok_or("school year")?,
            cohort: GradYear::CO2027,
            participation_rule: &fixture.profile.input().policies.participation,
        },
        interval,
        boundary_proof: fixture.proof("in-season-day")?,
    })
}

#[test]
fn v4_document_digest_and_explicit_dispatch_preserve_v3() -> TestResult {
    let fixture = fixture()?;
    let document: serde_json::Value = serde_json::from_str(PARTICIPATION_POLICY_V4_DOCUMENT)?;
    check!(eq; serde_json::to_string(&document)?, PARTICIPATION_POLICY_V4_DOCUMENT);
    check!(eq; format!("{:x}", Sha256::digest(PARTICIPATION_POLICY_V4_DOCUMENT)), PARTICIPATION_POLICY_V4_SHA256);
    check!(eq; participation_policy_v3()?.digest(), "2068e3a252ef111b104117333dcf426e904d89d514fdaa28af9607cd51727882");
    let case = fixture.case()?;
    let observation = case.observation(&fixture);
    check!(matches!(
        assess_participation_for_policy(fixture.query()?, &observation),
        Err(ParticipationInputError::UnsupportedPolicy)
    ));
    let old = Fixture::new()?;
    let old_case = old.case()?;
    let old_observation = old_case.observation(&old);
    check!(matches!(
        assess_participation_with_in_season(
            old.query()?,
            &old_observation,
            InSeasonEvidence::Unavailable
        ),
        Err(ParticipationInputError::UnsupportedPolicy)
    ));
    Ok(())
}

#[test]
fn source_proven_day_admits_without_inventing_summer_or_status() -> TestResult {
    let fixture = fixture()?;
    let day_window = range(fixture.date.as_str(), fixture.date.as_str())?;
    let facts = [fact(&fixture, &day_window)?];
    let mut case = fixture.case()?;
    case.windows.clear();
    let observation = case.observation(&fixture);
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Uncontested(&facts),
    )?;
    check!(eq; result.outcome, CandidateOutcome::EligibleCandidate);
    check!(result.reasons.is_empty());
    let absent = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Unavailable,
    )?;
    check!(eq; absent.reasons.iter().collect::<Vec<_>>(), vec![Reason::SummerBoundaryUnresolved]);
    case.statuses.clear();
    let observation = case.observation(&fixture);
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Uncontested(&facts),
    )?;
    check!(eq; result.reasons.iter().collect::<Vec<_>>(), vec![Reason::HighSchoolStatusMissing]);
    Ok(())
}

#[test]
fn disjoint_supported_days_union_but_later_day_remains_unresolved() -> TestResult {
    let fixture = fixture()?;
    let first = range("2026-07-30", "2026-07-30")?;
    let second = range("2026-07-31", "2026-07-31")?;
    let mut later_fact = fact(&fixture, &second)?;
    later_fact.boundary_proof = fixture.proof("second-season-day")?;
    let facts = [fact(&fixture, &first)?, later_fact];
    let mut case = fixture.case()?;
    case.windows.clear();
    let observation = case.observation(&fixture);
    check!(eq; assess_participation_with_in_season(fixture.query()?, &observation, InSeasonEvidence::Uncontested(&facts))?.outcome, CandidateOutcome::EligibleCandidate);
    let later = day("2026-08-01")?;
    case.date = PerformanceDateFact::Exact(&later);
    let observation = case.observation(&fixture);
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Uncontested(&facts),
    )?;
    check!(eq; result.outcome, CandidateOutcome::Unresolved);
    check!(eq; result.reasons.iter().collect::<Vec<_>>(), vec![Reason::SummerBoundaryUnresolved]);
    Ok(())
}

#[test]
fn in_season_proof_does_not_override_full_window_or_conflict() -> TestResult {
    let fixture = fixture()?;
    let interval = range("2026-07-31", "2026-07-31")?;
    let facts = [fact(&fixture, &interval)?];
    let outside = range("2025-09-01", "2026-07-30")?;
    let mut case = fixture.case()?;
    let window = case.windows.first_mut().ok_or("window")?;
    window.interval = &outside;
    window.documented_summer_end = outside.through();
    let observation = case.observation(&fixture);
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Uncontested(&facts),
    )?;
    check!(eq; result.outcome, CandidateOutcome::Excluded);
    check!(eq; result.reasons.iter().collect::<Vec<_>>(), vec![Reason::OutsideParticipationWindow]);
    case.windows.clear();
    let mut negative = fact(&fixture, &interval)?;
    negative.boundary_proof = fixture.proof("season-negative")?;
    let contrary = [negative];
    let observation = case.observation(&fixture);
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Conflicted {
            supported: &facts,
            contrary: &contrary,
        },
    )?;
    check!(eq; result.outcome, CandidateOutcome::Unresolved);
    check!(result.reasons.contains(Reason::ParticipationWindowConflict));
    Ok(())
}

#[test]
fn in_season_proofs_share_subject_identity_and_fact_bounds() -> TestResult {
    let fixture = fixture()?;
    let interval = range("2026-07-31", "2026-07-31")?;
    let case = fixture.case()?;
    let observation = case.observation(&fixture);
    let duplicate = [fact(&fixture, &interval)?];
    check!(matches!(
        assess_participation_with_in_season(
            fixture.query()?,
            &observation,
            InSeasonEvidence::Conflicted {
                supported: &duplicate,
                contrary: &duplicate
            }
        ),
        Err(ParticipationInputError::ConflictingProofIdentity)
    ));
    let facts = vec![fact(&fixture, &interval)?; MAX_PARTICIPATION_FACTS];
    check!(matches!(
        assess_participation_with_in_season(
            fixture.query()?,
            &observation,
            InSeasonEvidence::Uncontested(&facts)
        ),
        Err(ParticipationInputError::TooManyFacts)
    ));
    let foreign = crate::model::Id::mint("ath", &["foreign-season-subject"]);
    let mut wrong = fact(&fixture, &interval)?;
    wrong.boundary_proof.athlete = &foreign;
    check!(matches!(
        assess_participation_with_in_season(
            fixture.query()?,
            &observation,
            InSeasonEvidence::Uncontested(&[wrong])
        ),
        Err(ParticipationInputError::ForeignSubject)
    ));
    Ok(())
}

#[test]
fn invisible_contrary_and_duplicate_positive_facts_preserve_frozen_eligibility() -> TestResult {
    let fixture = fixture()?;
    let interval = range("2026-07-31", "2026-07-31")?;
    let positive = fact(&fixture, &interval)?;
    let mut negative = positive;
    negative.boundary_proof = fixture.proof("future-contrary")?;
    negative.boundary_proof.known = crate::affiliation_candidates::KnownBounds::new(
        fixture::object("store-generation")?,
        fixture::revision(8)?,
        fixture::revision(9)?,
    )?;
    let mut case = fixture.case()?;
    case.windows.clear();
    let observation = case.observation(&fixture);
    for supported in [vec![positive], vec![positive, positive]] {
        let contrary = [negative, negative];
        let result = assess_participation_with_in_season(
            fixture.query()?,
            &observation,
            InSeasonEvidence::Conflicted {
                supported: &supported,
                contrary: &contrary,
            },
        )?;
        check!(eq; result.outcome, CandidateOutcome::EligibleCandidate);
        check!(result.reasons.is_empty());
    }
    let changed_interval = range("2026-07-30", "2026-07-31")?;
    let mut altered = positive;
    altered.interval = &changed_interval;
    check!(matches!(
        assess_participation_with_in_season(
            fixture.query()?,
            &observation,
            InSeasonEvidence::Uncontested(&[positive, altered])
        ),
        Err(ParticipationInputError::ConflictingProofIdentity)
    ));
    Ok(())
}

#[test]
fn season_denial_does_not_override_a_documented_full_eligibility_window() -> TestResult {
    let fixture = fixture()?;
    let interval = range("2026-07-31", "2026-07-31")?;
    let mut positive = fact(&fixture, &interval)?;
    positive.boundary_proof.known = crate::affiliation_candidates::KnownBounds::new(
        fixture::object("store-generation")?,
        fixture::revision(8)?,
        fixture::revision(9)?,
    )?;
    let mut negative = fact(&fixture, &interval)?;
    negative.boundary_proof = fixture.proof("outside-school-season")?;
    let case = fixture.case()?;
    let observation = case.observation(&fixture);
    let supported = [positive];
    let contrary = [negative];
    let result = assess_participation_with_in_season(
        fixture.query()?,
        &observation,
        InSeasonEvidence::Conflicted {
            supported: &supported,
            contrary: &contrary,
        },
    )?;
    check!(eq; result.outcome, CandidateOutcome::EligibleCandidate);
    check!(result.reasons.is_empty());
    Ok(())
}

mod guards;
