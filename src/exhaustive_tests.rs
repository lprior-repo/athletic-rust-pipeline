use crate::{
    config::Config,
    exhaustive_engine::Engine,
    exhaustive_identity,
    model::{Candidate, ModelDecision, Prospect},
};
use anyhow::{Context, Result};
use mockito::{Matcher, Server};
use serde_json::json;

fn prospect() -> Prospect {
    Prospect {
        source_key: "Export:2".to_owned(),
        sheet: "Export".to_owned(),
        excel_row: 2,
        first_name: "Ada".to_owned(),
        last_name: "Runner".to_owned(),
        school: "Central High".to_owned(),
        city: "Austin".to_owned(),
        state: "TX".to_owned(),
        sport: "Basketball".to_owned(),
        expected_graduation_year: Some(2027),
        source_fields: [("Person Email".to_owned(), "ada@example.test".to_owned())].into(),
    }
}

fn config(search: &Server, extractor: &Server, reviewer: &Server) -> Result<Config> {
    let mut config: Config = toml::from_str(include_str!("../config.exhaustive.toml"))?;
    config.discovery.athletic_search_url = search.url();
    config.discovery.search_delay_ms = 0;
    config.discovery.max_attempts = 1;
    config.ollama.url = extractor.url();
    config
        .identity_review
        .as_mut()
        .context("review config")?
        .url = reviewer.url();
    Ok(config)
}

fn model_body(value: serde_json::Value) -> String {
    json!({"choices":[{"message":{"content": value.to_string()}}]}).to_string()
}

async fn search_mock(server: &mut Server, sport: &str) -> mockito::Mock {
    let (filter, path) = if sport == "tf" {
        ("t:a a:tf", "track-and-field")
    } else {
        ("t:a a:xc", "cross-country")
    };
    server.mock("POST", "/").match_body(Matcher::PartialJson(json!({"fq":filter})))
        .with_body(json!({"d":{"count":1,"pager":"","results":format!("<tr><td><a href=\"/athlete/7/{path}\">Ada Runner</a> Central High Austin TX Class of 2027</td></tr>")}}).to_string())
        .create_async().await
}

#[tokio::test]
async fn given_basketball_source_when_both_sport_filters_find_same_id_then_match_preserves_source_fields(
) -> Result<()> {
    let (mut search, mut extractor, mut reviewer) = (
        Server::new_async().await,
        Server::new_async().await,
        Server::new_async().await,
    );
    let _tf = search_mock(&mut search, "tf").await;
    let _xc = search_mock(&mut search, "xc").await;
    let _extract = extractor.mock("POST", "/v1/chat/completions")
        .with_body(model_body(json!({"athlete_name":"Ada Runner","school":"Central High","location":"Austin TX","graduation_year":2027,"sports":["Track & Field","Cross Country"],"marks":null})))
        .create_async().await;
    let _review = reviewer.mock("POST", "/v1/chat/completions")
        .with_body(model_body(json!({"decision":"MATCH","candidate_index":0,"confidence":0.99,"reason":"Name school location and year agree","track_confirmed":true,"xc_confirmed":true})))
        .create_async().await;
    let config = config(&search, &extractor, &reviewer)?;
    let dir = tempfile::tempdir()?;
    let record = Engine::new(config, dir.path(), false)?
        .process(&prospect())
        .await?;

    assert_eq!(record.status, "MATCH");
    assert_eq!(record.prospect.sport, "Basketball");
    assert!(record.track_confirmed && record.xc_confirmed);
    assert_eq!(record.prospect.source_fields, prospect().source_fields);
    assert_eq!(record.candidates.len(), 2);
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.profile_url.contains("/athlete/7/track-and-field")));
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.profile_url.contains("/athlete/7/cross-country")));
    assert!(record.candidates.iter().all(|candidate| {
        candidate
            .sports
            .iter()
            .any(|sport| sport == "Track & Field")
            && candidate
                .sports
                .iter()
                .any(|sport| sport == "Cross Country")
    }));
    Ok(())
}

#[tokio::test]
async fn given_model_failure_when_extraction_runs_then_error_is_not_an_identity_rejection(
) -> Result<()> {
    let (mut search, mut extractor, reviewer) = (
        Server::new_async().await,
        Server::new_async().await,
        Server::new_async().await,
    );
    let _tf = search_mock(&mut search, "tf").await;
    let _xc = search_mock(&mut search, "xc").await;
    let _failure = extractor
        .mock("POST", "/v1/chat/completions")
        .with_status(503)
        .create_async()
        .await;
    let dir = tempfile::tempdir()?;
    let record = Engine::new(config(&search, &extractor, &reviewer)?, dir.path(), false)?
        .process(&prospect())
        .await?;
    assert_eq!(record.status, "AI_ERROR");
    assert_eq!(record.model_decision.decision, "ERROR");
    assert!(record.ai_logic.contains("503"));
    assert!(record.selected_profile_url.is_empty());
    Ok(())
}
#[tokio::test]
async fn given_search_failure_when_discovery_runs_then_error_is_fail_closed() -> Result<()> {
    let (mut search, extractor, reviewer) = (
        Server::new_async().await,
        Server::new_async().await,
        Server::new_async().await,
    );
    let _failure = search
        .mock("POST", "/")
        .with_status(503)
        .create_async()
        .await;
    let dir = tempfile::tempdir()?;
    let record = Engine::new(config(&search, &extractor, &reviewer)?, dir.path(), false)?
        .process(&prospect())
        .await?;
    assert_eq!(record.status, "SEARCH_ERROR");
    assert_eq!(record.model_decision.decision, "ERROR");
    assert!(record.ai_logic.contains("503"));
    assert!(record.selected_profile_url.is_empty());
    Ok(())
}

#[tokio::test]
async fn given_completed_cache_when_servers_are_unreachable_then_resume_uses_only_cache(
) -> Result<()> {
    let (mut search, mut extractor, mut reviewer) = (
        Server::new_async().await,
        Server::new_async().await,
        Server::new_async().await,
    );
    let _tf = search_mock(&mut search, "tf").await;
    let _xc = search_mock(&mut search, "xc").await;
    let _extract = extractor.mock("POST", "/v1/chat/completions")
        .with_body(model_body(json!({"athlete_name":"Ada Runner","school":"Central High","location":"Austin TX","graduation_year":2027,"sports":["Track & Field","Cross Country"],"marks":null})))
        .create_async().await;
    let _review = reviewer.mock("POST", "/v1/chat/completions")
        .with_body(model_body(json!({"decision":"MATCH","candidate_index":0,"confidence":0.99,"reason":"Name school location and year agree","track_confirmed":true,"xc_confirmed":true})))
        .create_async().await;
    let config = config(&search, &extractor, &reviewer)?;
    let dir = tempfile::tempdir()?;
    let first = Engine::new(config.clone(), dir.path(), false)?
        .process(&prospect())
        .await?;
    assert_eq!(first.status, "MATCH");

    drop((_tf, _xc, _extract, _review));
    drop((search, extractor, reviewer));

    let resumed = Engine::new(config, dir.path(), false)?
        .process(&prospect())
        .await?;
    assert_eq!(resumed.status, "MATCH");
    assert_eq!(resumed.selected_profile_url, first.selected_profile_url);
    assert_eq!(resumed.prospect.source_fields, first.prospect.source_fields);
    Ok(())
}

#[test]
fn given_different_athlete_tie_when_ai_matches_then_attribution_is_withheld() -> Result<()> {
    let config: Config = toml::from_str(include_str!("../config.exhaustive.toml"))?;
    let candidates = [7, 8]
        .into_iter()
        .map(|id| Candidate {
            profile_url: format!("https://www.athletic.net/athlete/{id}/track-and-field"),
            deterministic_score: 0.99,
            corroborated: true,
            sports: vec!["Track & Field".to_owned()],
            ..Default::default()
        })
        .collect();
    let decision = ModelDecision {
        decision: "MATCH".to_owned(),
        candidate_index: Some(0),
        confidence: 0.99,
        track_confirmed: true,
        model_status: "ok".to_owned(),
        ..Default::default()
    };
    let record =
        exhaustive_identity::finalize(prospect(), candidates, decision, &config.matching, 0.03);
    assert_eq!(record.status, "REVIEW");
    assert!(record.selected_profile_url.is_empty());
    assert!(!record.track_confirmed && !record.xc_confirmed);
    assert!(record.best_marks.is_empty());
    Ok(())
}

#[tokio::test]
async fn no_ai_never_contacts_models_and_searches_all_stages() -> Result<()> {
    let (mut search, mut extractor, mut reviewer) = (
        Server::new_async().await,
        Server::new_async().await,
        Server::new_async().await,
    );
    let _tf = search_mock(&mut search, "tf").await;
    let _xc = search_mock(&mut search, "xc").await;
    let forbidden_extract = extractor
        .mock("POST", "/v1/chat/completions")
        .expect(0)
        .create_async()
        .await;
    let forbidden_review = reviewer
        .mock("POST", "/v1/chat/completions")
        .expect(0)
        .create_async()
        .await;
    let dir = tempfile::tempdir()?;
    let record = Engine::new(config(&search, &extractor, &reviewer)?, dir.path(), true)?
        .process(&prospect())
        .await?;
    assert_eq!(record.status, "MATCH");
    assert_eq!(record.model_decision.model_status, "not_run_deterministic");
    assert!(record.track_confirmed && record.xc_confirmed);
    let cache = crate::search_cache::load_latest(&dir.path().join("search-cache.jsonl"))?;
    assert_eq!(cache.len(), 12);
    forbidden_extract.assert_async().await;
    forbidden_review.assert_async().await;
    Ok(())
}

#[test]
fn same_name_and_school_in_conflicting_states_cannot_be_confirmed() -> Result<()> {
    let cfg: Config = toml::from_str(include_str!("../config.exhaustive.toml"))?;
    let person = Prospect {
        first_name: "Steve".to_owned(),
        last_name: "Johnson".to_owned(),
        city: "Portland".to_owned(),
        state: "Oregon".to_owned(),
        ..prospect()
    };
    let candidates = [(7, "Portland, Oregon"), (8, "Portland, Michigan")]
        .into_iter()
        .map(|(id, location)| {
            let mut candidate = Candidate {
                profile_url: format!("https://www.athletic.net/athlete/{id}/track-and-field"),
                athlete_name: "Steve Johnson".to_owned(),
                school: person.school.clone(),
                location: location.to_owned(),
                graduation_year: Some(2027),
                sports: vec!["Track & Field".to_owned()],
                ..Default::default()
            };
            crate::scoring::score_athletics_candidate(&person, &mut candidate, &cfg.matching);
            candidate
        })
        .collect::<Vec<_>>();
    assert!(
        !candidates
            .get(1)
            .context("Michigan candidate")?
            .corroborated
    );
    let decision = exhaustive_identity::deterministic_decision(
        &candidates,
        &cfg.matching,
        cfg.discovery.ambiguity_margin,
    );
    assert_eq!(decision.candidate_index, Some(0));
    assert_eq!(decision.decision, "MATCH");
    let wrong = vec![candidates.get(1).context("Michigan candidate")?.clone()];
    let guess = exhaustive_identity::deterministic_decision(
        &wrong,
        &cfg.matching,
        cfg.discovery.ambiguity_margin,
    );
    let result = exhaustive_identity::finalize(
        person,
        wrong,
        guess,
        &cfg.matching,
        cfg.discovery.ambiguity_margin,
    );
    assert_eq!(result.status, "REVIEW");
    assert!(result.selected_profile_url.is_empty());
    Ok(())
}

#[test]
fn deterministic_extraction_keeps_observed_state_instead_of_expected_address() -> Result<()> {
    let cfg: Config = toml::from_str(include_str!("../config.exhaustive.toml"))?;
    let person = Prospect {
        state: "Oregon".to_owned(),
        ..prospect()
    };
    let hit = crate::model::SearchHit {
        title: person.full_name(),
        snippet: "Ada Runner Central High Portland, Michigan. Class of 2027".to_owned(),
        url: "https://www.athletic.net/athlete/7/track-and-field".to_owned(),
        ..Default::default()
    };
    let mut candidate =
        crate::extract::candidate_from_evidence_deterministic(&person, &hit, None, 4000);
    assert_eq!(candidate.location, "MI");
    crate::scoring::score_athletics_candidate(&person, &mut candidate, &cfg.matching);
    assert!(!candidate.corroborated);
    assert_eq!(
        crate::scoring::location_state("Huntington, West Virginia").as_deref(),
        Some("WV")
    );
    Ok(())
}
