use census_domain::model::{CanonicalSchool, ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::Table;

use super::support::{
    add_school, audit, batch, client, lane, lane_with, options, row, state, Fixture, TestResult,
};
use crate::run_lanes;

#[test]
fn exact_dual_advice_replays_without_reasking_or_writing_another_checkpoint() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec![good])?;
            let clients = [client(&first)?, client(&second)?];
            let report = run_lanes(&fixture.store, &clients, &options(), "first").await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.accepted, 1);
            let original = row(&fixture.store)?;
            let replay = run_lanes(&fixture.store, &clients, &options(), "second").await?;
            check!(eq; replay.requested, 0);
            check!(eq; replay.failed, 0);
            check!(eq; row(&fixture.store)?, original);
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            check!(eq; fixture.store.receipt_count()?, 1);
            Ok(())
        })
}

#[test]
fn historical_single_model_acceptance_is_reasked_and_cannot_survive_disagreement() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, mut case) = Fixture::school()?;
            case.state = ReviewState::Resolved;
            fixture
                .store
                .replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
            let historical = ReviewVerdictRecord {
                id: case.id.clone(),
                case_id: case.id.clone(),
                subject_id: case.subject_id.clone(),
                family: case.family.clone(),
                member_ids: case.member_ids.clone(),
                kind: "value_proposed".to_string(),
                field: "state".to_string(),
                value: "WI".to_string(),
                accepted: true,
                confidence: 99,
                rationale: "one model guessed".to_string(),
                reviewer: "historical-model".to_string(),
                observed_at: "old".to_string(),
            };
            fixture
                .store
                .replace_many(Table::IdentityVerdicts, std::slice::from_ref(&historical))?;
            let (first, server_a) = lane(vec![batch(&case, "value_proposed", "state", "WI")])?;
            let (second, server_b) = lane(vec![batch(&case, "value_proposed", "state", "MN")])?;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options(),
                "cutover",
            )
            .await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.requested, 1);
            check!(eq; report.accepted, 0);
            check!(!row(&fixture.store)?.accepted);
            check!(eq; audit(&fixture.store)?["outcome"], "disagreement");
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            Ok(())
        })
}

#[test]
fn changed_evidence_invalidates_standing_advice_and_binds_the_new_packet() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone(), good.clone()])?;
            let (second, server_b) =
                lane(vec![good, batch(&case, "value_proposed", "state", "MN")])?;
            let clients = [client(&first)?, client(&second)?];
            check!(eq; run_lanes(&fixture.store, &clients, &options(), "first").await?.accepted, 1);
            let old_digest = audit(&fixture.store)?["evidence_digest"].clone();
            let mut schools = fixture.store.scan::<CanonicalSchool>(Table::Schools)?;
            schools[0].athletics_website = Some("https://new-source.example/school".to_string());
            fixture.store.append_many(Table::Schools, &schools)?;
            let report = run_lanes(&fixture.store, &clients, &options(), "second").await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.requested, 1);
            check!(eq; report.accepted, 0);
            check!(ne; audit(&fixture.store)?["evidence_digest"], old_digest);
            check!(eq; audit(&fixture.store)?["outcome"], "disagreement");
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            check!(eq; fixture.store.receipt_count()?, 3);
            Ok(())
        })
}

#[test]
fn evidence_changed_during_advice_prevents_acceptance_of_the_old_snapshot() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let store = fixture.store.clone();
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane_with(vec![good.clone()], move |_, _| {
                let mut schools = store.scan::<CanonicalSchool>(Table::Schools)?;
                schools[0].athletics_website =
                    Some("https://new-source.example/school".to_string());
                store.append_many(Table::Schools, &schools)?;
                Ok(())
            })?;
            let (second, server_b) = lane(vec![good])?;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options(),
                "stale",
            )
            .await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.accepted, 0);
            check!(eq; audit(&fixture.store)?["outcome"], "evidence_changed");
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            check!(!row(&fixture.store)?.accepted);
            check!(audit(&fixture.store)?["packet"]["evidence"]
                .as_array()
                .ok_or("original packet")?
                .iter()
                .any(|fact| fact["field"] == "association" && fact["value"] == "WIAA"));
            Ok(())
        })
}

#[test]
fn cached_cases_do_not_consume_the_budget_for_new_ambiguous_cases() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, first_case) = Fixture::school()?;
            let second_case = add_school(&fixture.store, "Madison East")?;
            let mut cases = [first_case, second_case];
            cases.sort_by(|first, second| first.id.cmp(&second.id));
            let replies = cases
                .iter()
                .map(|case| batch(case, "value_proposed", "state", "WI"))
                .collect::<Vec<_>>();
            let (first, server_a) = lane(replies.clone())?;
            let (second, server_b) = lane(replies)?;
            let clients = [client(&first)?, client(&second)?];
            let mut options = options();
            options.limit = 1;
            let first = run_lanes(&fixture.store, &clients, &options, "first").await?;
            let second = run_lanes(&fixture.store, &clients, &options, "second").await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; first.accepted, 1);
            check!(eq; second.requested, 1);
            check!(eq; second.accepted, 1);
            check!(fixture
                .store
                .scan::<ReviewCase>(Table::ReviewCases)?
                .iter()
                .all(|case| case.state == ReviewState::Resolved));
            check!(eq; fixture.store.receipt_count()?, 2);
            Ok(())
        })
}

#[test]
fn missing_subjects_are_durable_unresolved_review_not_silent_success() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let missing = ReviewCase::pending(
                "School jurisdiction unresolved",
                "missing-school",
                "Unknown",
                "missing source subject",
            );
            fixture.store.replace_many(Table::ReviewCases, &[missing])?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec![good])?;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options(),
                "missing",
            )
            .await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.requested, 2);
            check!(eq; report.accepted, 1);
            check!(eq; report.unaskable, 1);
            check!(eq; report.unanswered, 0);
            let rows = fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
            let missing = rows
                .iter()
                .find(|row| row.subject_id == "missing-school")
                .ok_or("missing finding")?;
            check!(!missing.accepted);
            let audit: serde_json::Value = serde_json::from_str(&missing.rationale)?;
            check!(eq; audit["outcome"], "missing_subject");
            check!(eq; audit["packet"], serde_json::Value::Null);
            Ok(())
        })
}

#[test]
fn a_case_superseded_during_advice_is_not_resurrected_by_the_checkpoint() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async { let (fixture, case) = Fixture::school()?;
let store = fixture.store.clone();
let mut superseded = case.clone();
superseded.state = ReviewState::Superseded;
let good = batch(&case, "value_proposed", "state", "WI");
let (first, server_a) = lane_with(vec![good.clone()], move |_, _| {
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&superseded))?;
    Ok(())
})?;
let (second, server_b) = lane(vec![good])?;
let error = match run_lanes(
    &fixture.store,
    &[client(&first)?, client(&second)?],
    &options(),
    "superseded",
)
.await {
    Err(error) => error,
    Ok(_) => return Err("stale case checkpoint rejected".into()),
};
server_a.join().map_err(|_| "first lane panicked")??;
server_b.join().map_err(|_| "second lane panicked")??;
check!(matches!(error, census_store::StoreError::Invariant { detail } if detail.contains("changed before checkpoint")));
check!(eq; state(&fixture.store)?, ReviewState::Superseded);
check!(fixture.store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?.is_empty());
check!(eq; fixture.store.receipt_count()?, 0);
Ok(()) })
}
