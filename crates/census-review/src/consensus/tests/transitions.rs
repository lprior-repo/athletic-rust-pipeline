use super::support::{
    add_school, audit, batch, client, lane, options, row, state, Fixture, TestResult,
};
use crate::{run_lanes, ModelClient, ModelResponseFormat};
use census_domain::model::{CanonicalSchool, ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::Table;

fn changed_format(client: &ModelClient) -> TestResult<ModelClient> {
    Ok(ModelClient::new(
        client
            .options()
            .clone()
            .with_response_format(ModelResponseFormat::PromptJson),
    )?)
}

#[test]
fn returning_to_prior_advice_binding_applies_the_new_transition_on_the_same_date() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone(), good.clone(), good.clone()])?;
            let (second, server_b) = lane(vec![
                good.clone(),
                batch(&case, "value_proposed", "state", "MN"),
                good,
            ])?;
            let original = [client(&first)?, client(&second)?];
            let initial = run_lanes(&fixture.store, &original, &options(), "one-date").await?;
            let initial_audit = audit(&fixture.store)?;
            let changed = [changed_format(&original[0])?, changed_format(&original[1])?];
            let disagreement = run_lanes(&fixture.store, &changed, &options(), "one-date").await?;
            check!(!row(&fixture.store)?.accepted);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            let retained_disagreement = row(&fixture.store)?;
            let restored = run_lanes(&fixture.store, &original, &options(), "one-date").await?;
            let requests_a = server_a.join().map_err(|_| "first lane panicked")??;
            let requests_b = server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; (initial.accepted, disagreement.accepted, restored.accepted), (1, 0, 1));
            check!(eq; requests_a.len(), 3);
            check!(eq; requests_b.len(), 3);
            check!(
                row(&fixture.store)?.accepted,
                "the prior receipt must not suppress the new state transition"
            );
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            check!(eq; audit(&fixture.store)?, initial_audit);
            check!(fixture
                .store
                .journal_payloads("review_advice_v1")?
                .contains(&serde_json::to_value(retained_disagreement)?));
            Ok(())
        })
}

#[test]
fn format_change_revokes_unasked_acceptance_beyond_the_model_call_budget() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async { let (fixture, first_case) = Fixture::school()?;
let second_case = add_school(&fixture.store, "Madison East")?;
let mut cases = [first_case, second_case];
cases.sort_by(|first, second| first.id.cmp(&second.id));
let replies = cases.iter().map(|case| batch(case, "value_proposed", "state", "WI")).collect::<Vec<_>>();
let mut three_replies = replies.clone();
three_replies.push(replies[0].clone());
let (first, server_a) = lane(three_replies)?;
let (second, server_b) = lane(replies)?;
let original = [client(&first)?, client(&second)?];
check!(eq; run_lanes(&fixture.store, &original, &options(), "first").await?.accepted, 2);
let previous = fixture.store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
let changed = [changed_format(&original[0])?, original[1].clone()];
let mut limited = options();
limited.limit = 1;
let report = run_lanes(&fixture.store, &changed, &limited, "cutover").await?;
check!(eq; server_a.join().map_err(|_| "first lane panicked")??.len(), 3);
check!(eq; server_b.join().map_err(|_| "second lane panicked")??.len(), 2);
check!(eq; (report.requested, report.accepted), (1, 1));
let rows = fixture.store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
let asked = rows.iter().find(|row| row.case_id == cases[0].id).ok_or("asked case")?;
let unasked = rows.iter().find(|row| row.case_id == cases[1].id).ok_or("unasked case")?;
check!(asked.accepted);
check!(!unasked.accepted, "stale standing advice must be revoked independently of call budget");
check!(eq; (unasked.field.as_str(), unasked.value.as_str()), ("", ""));
let persisted = fixture.store.scan::<ReviewCase>(Table::ReviewCases)?;
check!(eq; persisted.iter().find(|case| case.id == cases[1].id).ok_or("unasked state")?.state,
ReviewState::Retained);
let preserved = fixture.store.journal_payloads("review_invalidated_advice_v1")?;
for old in &previous {
    let old = serde_json::to_value(old)?;
    check!(preserved.iter().any(|payload| payload == &old));
}
Ok(()) })
}

#[test]
fn evidence_change_revokes_standing_acceptance_even_with_zero_model_call_budget() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec![good])?;
            let clients = [client(&first)?, client(&second)?];
            check!(eq; run_lanes(&fixture.store, &clients, &options(), "first").await?.accepted, 1);
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            let old = row(&fixture.store)?;
            let mut schools = fixture.store.scan::<CanonicalSchool>(Table::Schools)?;
            schools[0].athletics_website =
                Some("https://school.example/changed-evidence".to_string());
            fixture.store.append_many(Table::Schools, &schools)?;
            let mut limited = options();
            limited.limit = 0;
            let report = run_lanes(&fixture.store, &clients, &limited, "source-change").await?;
            check!(eq; (report.requested, report.failed), (0, 0));
            check!(!row(&fixture.store)?.accepted);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            check!(fixture
                .store
                .journal_payloads("review_invalidated_advice_v1")?
                .contains(&serde_json::to_value(old)?));
            Ok(())
        })
}
