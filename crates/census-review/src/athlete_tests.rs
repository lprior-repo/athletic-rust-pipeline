use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, RetainedConflict,
    ReviewCase, ReviewState, SourceIdentity, SourceNamespace, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use super::consensus::tests::support::{audit, batch, client, lane, row, state, TestResult};
use super::packets::pending_cases;
use super::{run_lanes, ReviewFamily, ReviewOptions};

#[path = "athlete_tests/binding_support.rs"]
mod binding_support;
#[path = "athlete_tests/cached_binding.rs"]
mod cached_binding;
#[path = "athlete_tests/contradictory_cohorts.rs"]
mod contradictory_cohorts;
#[path = "athlete_tests/inflight_binding.rs"]
mod inflight_binding;
#[path = "athlete_tests/retained_conflict.rs"]
mod retained_conflict;

struct Fixture {
    store: Store,
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> TestResult<(Self, ReviewCase)> {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let school = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Madison West High School",
            normalize_name("Madison West High School"),
            None,
        )
        .0
        .id;
        let boys = CanonicalAthlete::new(
            &school,
            "Jordan Smith",
            GradYear::CO2027,
            Gender::Boys,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
        );
        let girls = CanonicalAthlete::new(
            &school,
            "Jordan Smith",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
        );
        store.append_many(Table::Athletes, &[boys.clone(), girls.clone()])?;
        let detail = format!(
            "same school, name and cohort as every id here: {}, {}",
            boys.id, girls.id
        );
        let case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            boys.id.as_str(),
            "Jordan Smith (Madison West High School)",
            &detail,
        );
        let conflict = RetainedConflict::new(
            ATHLETE_IDENTITY_FAMILY,
            case.subject_id.as_str(),
            case.subject.as_str(),
            case.detail.as_str(),
        );
        store.replace_many(Table::Conflicts, &[conflict])?;
        Ok((Self { store, _dir: dir }, case))
    }
}

fn options() -> ReviewOptions {
    ReviewOptions {
        families: vec![ReviewFamily::AthleteIdentity],
        limit: 10,
        dry_run: false,
    }
}

#[test]
fn dual_same_person_agreement_cannot_override_a_gender_contradiction() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::new()?;
            let reply = batch(&case, "value_proposed", "identity", "same_person");
            let (first, server_a) = lane(vec![reply.clone()])?;
            let (second, server_b) = lane(vec![reply])?;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options(),
                "contradiction",
            )
            .await?;
            server_a
                .join()
                .map_err(|_| "first independent lane panicked")??;
            server_b
                .join()
                .map_err(|_| "second independent lane panicked")??;
            check!(eq; report.requested, 1);
            check!(eq; report.accepted, 0);
            check!(eq; report.rejected, 1);
            check!(eq; report.resolved(), 0);
            let verdict = row(&fixture.store)?;
            check!(eq; verdict.case_id, case.id);
            check!(eq; verdict.subject_id, case.subject_id);
            check!(eq; verdict.family, ATHLETE_IDENTITY_FAMILY);
            check!(!verdict.accepted);
            check!(eq; verdict.kind, "insufficient_evidence");
            let audit = audit(&fixture.store)?;
            check!(eq; audit["outcome"], "refused");
            check!(!audit["packet"]["evidence"]
                .as_array()
                .ok_or("evidence")?
                .iter()
                .any(|fact| fact["field"] == "flag"
                    && fact["value"]
                        .as_str()
                        .is_some_and(|value| value.starts_with("identity_corroborated:"))));
            let mut gender_differs = false;
            for fact in audit["packet"]["evidence"].as_array().ok_or("evidence")? {
                if fact["field"] == "flag"
                    && fact["value"]
                        .as_str()
                        .ok_or("flag")?
                        .starts_with("gender_differs:")
                {
                    gender_differs = true;
                    break;
                }
            }
            check!(gender_differs);
            for lane in audit["lanes"].as_array().ok_or("both lanes")? {
                check!(eq; lane["batch"]["verdicts"][0]["value"], "same_person");
            }
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            check!(eq; fixture.store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 2);
            Ok(())
        })
}

#[test]
fn agreed_different_person_advice_preserves_the_existing_contradiction_semantics() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::new()?;
            let reply = batch(&case, "value_proposed", "identity", "different_person");
            let (first, server_a) = lane(vec![reply.clone()])?;
            let (second, server_b) = lane(vec![reply])?;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options(),
                "contradiction",
            )
            .await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.accepted, 1);
            check!(eq; report.resolved(), 1);
            check!(eq; audit(&fixture.store)?["outcome"], "agreement");
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            check!(eq; row(&fixture.store)?.value, "different_person");
            check!(row(&fixture.store)?.accepted);
            check!(eq; fixture.store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 2);
            Ok(())
        })
}

#[test]
fn insufficient_and_invalid_identity_advice_are_retained_with_the_original_answers() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for (kind, value) in [
                ("insufficient_evidence", ""),
                ("value_proposed", "maybe_same"),
            ] {
                let (fixture, case) = Fixture::new()?;
                let reply = batch(&case, kind, "identity", value);
                let (first, server_a) = lane(vec![reply.clone()])?;
                let (second, server_b) = lane(vec![reply])?;
                let report = run_lanes(
                    &fixture.store,
                    &[client(&first)?, client(&second)?],
                    &options(),
                    value,
                )
                .await?;
                server_a.join().map_err(|_| "first lane panicked")??;
                server_b.join().map_err(|_| "second lane panicked")??;
                check!(eq; report.accepted, 0);
                check!(eq; report.resolved(), 0);
                check!(!row(&fixture.store)?.accepted);
                check!(eq; state(&fixture.store)?, ReviewState::Retained);
                let audit = audit(&fixture.store)?;
                check!(eq; audit["lanes"][0]["batch"]["verdicts"][0]["kind"], kind);
                check!(eq; audit["lanes"][1]["batch"]["verdicts"][0]["value"], value);
            }
            Ok(())
        })
}

#[test]
fn a_finding_held_by_both_the_conflict_and_the_review_table_is_selected_once() -> TestResult {
    let (fixture, case) = Fixture::new()?;
    fixture
        .store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    let pending = pending_cases(&fixture.store, &options())?;
    check!(eq; pending.len(), 1);
    check!(eq; pending[0].0.id, case.id);
    check!(eq; pending[0].1, ReviewFamily::AthleteIdentity);
    Ok(())
}

#[test]
fn dual_same_person_agreement_never_promotes_name_school_and_cohort_alone() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            let school = CanonicalSchool::new(
                UsJurisdiction::Wisconsin,
                "Madison West",
                "madison west",
                None,
            )
            .0
            .id;
            let first_athlete = CanonicalAthlete::new(
                &school,
                "Jordan Smith",
                GradYear::CO2027,
                Gender::Boys,
                SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
            );
            let second_athlete = CanonicalAthlete::new(
                &school,
                "Jordan Smith",
                GradYear::CO2027,
                Gender::Boys,
                SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1002"),
            );
            store.append_many(
                Table::Athletes,
                &[first_athlete.clone(), second_athlete.clone()],
            )?;
            let case = ReviewCase::pending(
                ATHLETE_IDENTITY_FAMILY,
                first_athlete.id.as_str(),
                "Jordan Smith (Madison West)",
                "name, school and cohort agree; provider objects do not",
            );
            store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
            let reply = batch(&case, "value_proposed", "identity", "same_person");
            let (first, server_a) = lane(vec![reply.clone()])?;
            let (second, server_b) = lane(vec![reply])?;
            let report = run_lanes(
                &store,
                &[client(&first)?, client(&second)?],
                &options(),
                "name-only",
            )
            .await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.accepted, 0);
            check!(eq; report.rejected, 1);
            check!(eq; state(&store)?, ReviewState::Retained);
            check!(eq; audit(&store)?["outcome"], "refused");
            check!(!row(&store)?.accepted);
            let retained = store.scan::<CanonicalAthlete>(Table::Athletes)?;
            check!(retained.contains(&first_athlete));
            check!(retained.contains(&second_athlete));
            Ok(())
        })
}
