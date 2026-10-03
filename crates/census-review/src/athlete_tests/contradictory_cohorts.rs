use census_domain::model::{
    CanonicalAthlete, Evidence, Grade, ObservedGrade, ReviewCase, ReviewPacket, ReviewState,
    SchoolYear, SourceRef, VerdictBatch,
};
use census_store::Table;
use serde_json::Value;

use super::binding_support::{assert_preserved, rows, Fixture};
use super::options;
use crate::athlete_verdict::HardContradiction;
use crate::consensus::tests::support::{audit, batch, client, lane, row, TestResult};
use crate::{run_lanes, triage, Adjudication, Admitted, Refusal, ReviewFamily, ReviewReport};

fn contradictory_rows() -> TestResult<Vec<CanonicalAthlete>> {
    let mut candidates = rows()?;
    for (athlete, owner) in candidates.iter_mut().zip(["1001", "1002"]) {
        athlete.observed_grades.clear();
        athlete.evidence.clear();
        for grade in [11, 10] {
            let source = SourceRef::new(
                format!("public-roster-{owner}-grade-{grade}"),
                Some(format!(
                    "https://fixture.test/rosters/{owner}/grade/{grade}"
                )),
            );
            athlete.observed_grades.push(ObservedGrade {
                grade: Grade::new(grade).ok_or("invalid fixture grade")?,
                school_year: SchoolYear::new(2025).ok_or("invalid fixture school year")?,
                source: source.clone(),
            });
            athlete
                .evidence
                .push(Evidence::parsed(source, "2026-10-01"));
        }
    }
    Ok(candidates)
}

async fn agree(fixture: &Fixture, answer: &str) -> TestResult<ReviewReport> {
    let reply = batch(&fixture.case, "value_proposed", "identity", answer);
    let (first, server_a) = lane(vec![reply.clone()])?;
    let (second, server_b) = lane(vec![reply])?;
    let report = run_lanes(
        &fixture.store,
        &[client(&first)?, client(&second)?],
        &options(),
        "contradictory-cohorts",
    )
    .await;
    let first_join = server_a.join();
    let second_join = server_b.join();
    first_join.map_err(|_| std::io::Error::other("first independent lane failed"))??;
    second_join.map_err(|_| std::io::Error::other("second independent lane failed"))??;
    Ok(report?)
}

fn assert_case(fixture: &Fixture, expected_state: ReviewState) -> TestResult {
    let mut expected = fixture.case.clone();
    expected.state = expected_state;
    check!(eq; fixture.store.scan::<ReviewCase>(Table::ReviewCases)?, vec![expected]);
    let verdict = row(&fixture.store)?;
    check!(eq; verdict.case_id, fixture.case.id);
    check!(eq; verdict.subject_id, fixture.case.subject_id);
    check!(eq; verdict.member_ids, fixture.case.member_ids);
    Ok(())
}

fn assert_retained_advice(retained: &Value, answer: &str, expected: Adjudication) -> TestResult {
    let packet: ReviewPacket = serde_json::from_value(retained["packet"].clone())?;
    let lanes = retained["lanes"]
        .as_array()
        .ok_or_else(|| std::io::Error::other("missing retained advice lanes"))?;
    check!(eq; lanes.len(), 2);
    for lane in lanes {
        check!(eq; lane["status"], "answered");
        check!(eq; lane["dropped"], 0);
        let advice: VerdictBatch = serde_json::from_value(lane["batch"].clone())?;
        check!(eq; advice.subject_id, packet.subject_id);
        let (verdicts, dropped) = triage(&packet, ReviewFamily::AthleteIdentity, advice);
        check!(eq; dropped, 0);
        check!(eq; verdicts.len(), 1);
        for (verdict, adjudication) in verdicts {
            check!(eq; verdict.value.as_deref(), Some(answer));
            check!(eq; adjudication, expected);
        }
    }
    Ok(())
}

#[test]
fn dual_same_person_agreement_cannot_accept_identical_contradictory_cohorts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let candidates = contradictory_rows()?;
            let fixture = Fixture::new(&candidates)?;
            let report = agree(&fixture, "same_person").await?;
            check!(eq; report.accepted, 0);
            check!(eq; report.resolved(), 0);
            check!(eq; report.rejected, 1);
            check!(eq; report.answered, 1);
            check!(eq; report.failed, 0);
            assert_case(&fixture, ReviewState::Retained)?;
            let verdict = row(&fixture.store)?;
            check!(!verdict.accepted);
            check!(eq; verdict.kind, "insufficient_evidence");
            let retained = audit(&fixture.store)?;
            check!(eq; retained["outcome"], "hard_contradiction");
            assert_retained_advice(
                &retained,
                "same_person",
                Adjudication::Refused(Refusal::HardContradiction(
                    HardContradiction::GradYearEvidenceDiffers,
                )),
            )?;
            assert_preserved(&fixture.store, &candidates)?;
            Ok(())
        })
}

#[test]
fn dual_different_person_agreement_accepts_identical_contradictory_cohorts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let candidates = contradictory_rows()?;
            let fixture = Fixture::new(&candidates)?;
            let report = agree(&fixture, "different_person").await?;
            check!(eq; report.accepted, 1);
            check!(eq; report.resolved(), 1);
            check!(eq; report.rejected, 0);
            check!(eq; report.answered, 1);
            check!(eq; report.failed, 0);
            assert_case(&fixture, ReviewState::Resolved)?;
            let verdict = row(&fixture.store)?;
            check!(verdict.accepted);
            check!(eq; verdict.value, "different_person");
            let retained = audit(&fixture.store)?;
            check!(eq; retained["outcome"], "agreement");
            assert_retained_advice(
                &retained,
                "different_person",
                Adjudication::Decided(Admitted {
                    field: "identity".to_string(),
                    value: "different_person".to_string(),
                }),
            )?;
            assert_preserved(&fixture.store, &candidates)?;
            Ok(())
        })
}
