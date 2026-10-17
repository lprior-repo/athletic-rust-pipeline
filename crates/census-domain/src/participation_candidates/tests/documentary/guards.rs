use super::*;
use crate::public_identity_history::ReplayLimits;

#[test]
fn documentary_ownership_rejects_foreign_performance_and_proof_athlete() -> TestResult {
    let f = v4_fixture()?;
    let document = Document::new()?;
    let foreign_performance = Id::mint("perf", &["foreign-documentary-performance"]);
    let foreign_athlete = Id::mint("ath", &["foreign-documentary-athlete"]);
    for change_performance in [true, false] {
        let mut case = f.case()?;
        case.ownership = document.ownership(&f)?;
        let ResultOwnership::DocumentaryBound {
            performance, proof, ..
        } = &mut case.ownership
        else {
            return Err("documentary ownership absent".into());
        };
        if change_performance {
            *performance = &foreign_performance;
        } else {
            proof.athlete = &foreign_athlete;
        }
        check!(matches!(
            rejected(&f, &case)?,
            ParticipationInputError::ForeignSubject
        ));
    }
    Ok(())
}

#[test]
fn documentary_ownership_requires_each_exact_interpretation_capture_and_locator_link() -> TestResult
{
    let f = v4_fixture()?;
    let document = Document::new()?;
    for field in 0..4 {
        let mut case = f.case()?;
        case.ownership = document.ownership(&f)?;
        let ResultOwnership::DocumentaryBound {
            source_result,
            proof,
            ..
        } = &mut case.ownership
        else {
            return Err("documentary ownership absent".into());
        };
        match field {
            0 => *source_result = object("different-source-result")?,
            1 => proof.interpretation = object("different-interpretation")?,
            2 => proof.capture = object("different-capture")?,
            3 => proof.locator = object("different-locator")?,
            _ => return Err("documentary link field absent".into()),
        }
        check!(matches!(
            rejected(&f, &case)?,
            ParticipationInputError::InvalidDocumentaryOwnership
        ));
    }
    Ok(())
}

#[test]
fn documentary_ownership_requires_result_row_role_and_bounded_nonempty_inventory() -> TestResult {
    let f = v4_fixture()?;
    for field in 0..5 {
        let mut document = Document::new()?;
        match field {
            0 => document.subject.role = AttachmentSubjectRoleV1::AthleteStatus,
            1 => document.subject.role = AttachmentSubjectRoleV1::CorroboratingRoster,
            2 => document.inventory.version = 2,
            3 => document.inventory.records = 0,
            4 => {
                document.inventory.records = u64::try_from(ReplayLimits::SMALL.records)?
                    .checked_add(1)
                    .ok_or("inventory test overflow")?
            }
            _ => return Err("documentary inventory field absent".into()),
        }
        let mut case = f.case()?;
        case.ownership = document.ownership(&f)?;
        check!(matches!(
            rejected(&f, &case)?,
            ParticipationInputError::InvalidDocumentaryOwnership
        ));
    }
    let mut document = Document::new()?;
    document.inventory.records = u64::try_from(ReplayLimits::SMALL.records)?;
    let mut case = f.case()?;
    case.ownership = document.ownership(&f)?;
    check!(eq; assessed(&f, &case)?, (CandidateOutcome::EligibleCandidate, vec![]));
    Ok(())
}

#[test]
fn documentary_ownership_is_not_reinterpreted_under_old_or_foreign_participation_policies(
) -> TestResult {
    let original = Fixture::new()?;
    let foreign = crate::model::ContractRefV1::new(1, 4, &"b".repeat(64))?;
    let policies = [
        original.profile.input().policies.participation.clone(),
        participation_policy_v2()?,
        participation_policy_v3()?,
        foreign,
    ];
    for policy in policies {
        let mut f = Fixture::new()?;
        let mut input = f.profile.input().clone();
        input.policies.participation = policy;
        f.profile = QueryProfileV1::new(input)?;
        f.profile_id = f.profile.id()?;
        let document = Document::new()?;
        let mut case = f.case()?;
        case.ownership = document.ownership(&f)?;
        let observation = case.observation(&f);
        let result = match f.profile.input().policies.participation.revision().get() {
            1 => assess_historical_participation(f.query()?, &observation),
            2 | 3 => assess_participation_for_policy(f.query()?, &observation),
            _ => assess_participation_with_in_season(
                f.query()?,
                &observation,
                InSeasonEvidence::Unavailable,
            ),
        };
        check!(matches!(
            result,
            Err(ParticipationInputError::UnsupportedPolicy)
        ));
    }
    Ok(())
}

#[test]
fn documentary_variant_does_not_relax_existing_provider_source_athlete_validation() -> TestResult {
    let f = v4_fixture()?;
    let mut invalid = f.source.clone();
    invalid.id.clear();
    let mut case = f.case()?;
    let ResultOwnership::Bound { source_athlete, .. } = &mut case.ownership else {
        return Err("provider ownership absent".into());
    };
    *source_athlete = &invalid;
    check!(matches!(
        rejected(&f, &case)?,
        ParticipationInputError::InvalidSourceAthlete
    ));
    Ok(())
}
