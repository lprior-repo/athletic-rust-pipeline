use super::{fixture::*, TestResult};
use crate::affiliation_candidates::*;
use crate::model::ProfileDateRangeV1;

#[test]
fn coop_membership_keeps_both_intervals_inclusive_and_preserves_exact_audit_dispositions(
) -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let campus = ProfileDateRangeV1::new(day("2026-09-01")?, day("2026-10-10")?)?;
    let program = ProfileDateRangeV1::new(day("2026-10-10")?, day("2027-06-15")?)?;
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.owner.target = OwnerTarget::Program(&fixture.program);
    raw.relationship = RelationshipCandidate::RepresentsCoopProgram {
        program: &fixture.program,
        member: CoopMemberCandidate::Claimed {
            school: &fixture.school,
            athlete_member_binding: key("synthetic-campus-binding")?,
            source_person: &fixture.person,
            campus_binding_interval: &campus,
            program_membership_interval: &program,
            membership_statement: SourceSpan::new(0, BODY.len())?,
            known: known(1, 3)?,
        },
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    let claim = claims.first().ok_or("claim")?;
    for date in ["2026-10-09", "2026-10-10", "2026-10-11"] {
        let as_of = day(date)?;
        let mut query = fixture.query()?;
        query.as_of = &as_of;
        let selected = select_candidates(query, &claims, &[])?;
        let (outcome, disposition) = match date {
            "2026-10-10" => (
                CandidateOutcome::SingleCandidate(claim),
                Disposition::Candidate,
            ),
            _ => (CandidateOutcome::Unknown, Disposition::CoopMemberUnknown),
        };
        check!(eq; selected.outcome, outcome);
        check!(eq; selected.facts,
        [FactDisposition::Assertion {
            fact: claim,
            disposition
        }]);
    }
    Ok(())
}

#[test]
fn visible_withdrawal_precedes_unknown_coop_member_in_the_assertion_audit() -> TestResult {
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
    let claim = claims.first().ok_or("claim")?;
    let changes = [fixture.change(
        &evidence,
        claim.raw().id,
        ChangeKind::WithdrawsClaim {
            scope: &fixture.interval,
        },
    )?];
    let change = changes.first().ok_or("change")?;
    let selected = select_candidates(fixture.query()?, &claims, &changes)?;
    check!(eq; selected.outcome, CandidateOutcome::Unknown);
    check!(eq; selected.facts,
    [
        FactDisposition::Assertion {
            fact: claim,
            disposition: Disposition::WithdrawnClaim
        },
        FactDisposition::Change {
            fact: change,
            disposition: Disposition::ChangeApplied
        },
    ]);
    Ok(())
}
