use super::*;
use crate::config::DiscoveryConfig;
use anyhow::Result;
use mockito::{Matcher, Server};
use serde_json::json;

fn config(server: &Server) -> DiscoveryConfig {
    DiscoveryConfig {
        athletic_search_url: server.url(),
        max_candidates: 1,
        request_timeout_seconds: 2,
        search_delay_ms: 0,
        max_attempts: 1,
        circuit_breaker_threshold: 1,
        ambiguity_margin: 0.03,
    }
}

fn row(id: usize) -> String {
    format!(r#"<tr><td><a href="/athlete/{id}/track-and-field">Ada Runner</a></td></tr>"#)
}

#[tokio::test]
async fn follows_pager_and_retains_candidates_beyond_legacy_cap() -> Result<()> {
    let mut server = Server::new_async().await;
    let first = server.mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"q":"Ada Runner","fq":"t:a a:tf","start":0})))
        .with_header("content-type", "application/json")
        .with_body(json!({"d":{"count":3,"results":format!("{}{}",row(1),row(2)),"pager":"<li data-start='2'><a>Next</a></li>"}}).to_string())
        .create_async()
        .await;
    let second = server
        .mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"start":2})))
        .with_header("content-type", "application/json")
        .with_body(json!({"d":{"count":3,"results":row(3),"pager":""}}).to_string())
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Ada Runner", "a:tf", 0))
        .await?;
    assert_eq!(
        result
            .hits
            .iter()
            .map(|hit| hit.url.as_str())
            .collect::<Vec<_>>(),
        vec![
            "https://www.athletic.net/athlete/1/track-and-field",
            "https://www.athletic.net/athlete/2/track-and-field",
            "https://www.athletic.net/athlete/3/track-and-field",
        ]
    );
    first.assert_async().await;
    second.assert_async().await;
    Ok(())
}

#[tokio::test]
async fn incomplete_or_invalid_search_never_becomes_successful_empty_result() -> Result<()> {
    let mut server = Server::new_async().await;
    let response = server
        .mock("POST", "/")
        .with_header("content-type", "application/json")
        .with_body(json!({"d":{"count":2,"results":row(1),"pager":""}}).to_string())
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Ada Runner", "a:tf", 0))
        .await;
    assert!(result.is_err());
    response.assert_async().await;
    Ok(())
}

#[tokio::test]
async fn explicit_zero_count_is_a_completed_empty_search() -> Result<()> {
    let mut server = Server::new_async().await;
    let response = server
        .mock("POST", "/")
        .with_header("content-type", "application/json")
        .with_body(json!({"d":{"count":0,"results":"","pager":""}}).to_string())
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Nobody Here", "a:xc", 0))
        .await?;
    assert!(result.hits.is_empty());
    response.assert_async().await;
    Ok(())
}

#[tokio::test]
async fn oversized_success_body_is_rejected_before_json_decode() -> Result<()> {
    let mut server = Server::new_async().await;
    let limit = usize::try_from(super::super::MAX_RESPONSE_BODY_BYTES)?;
    let response = server
        .mock("POST", "/")
        .with_header("content-type", "application/json")
        .with_body(json!({"d":{"count":0,"results":"x".repeat(limit),"pager":""}}).to_string())
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Ada Runner", "a:tf", 0))
        .await;
    let error = match result {
        Ok(_) => anyhow::bail!("oversized search body unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(error.message.contains("exceeds"));
    response.assert_async().await;
    Ok(())
}

#[tokio::test]
async fn malformed_athlete_id_is_an_incomplete_search() -> Result<()> {
    let mut server = Server::new_async().await;
    let response = server
        .mock("POST", "/")
        .with_header("content-type", "application/json")
        .with_body(
            json!({"d":{"count":1,"results":r#"<tr><td><a href="/athlete//track-and-field"></a></td></tr>"#,"pager":""}})
                .to_string(),
        )
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Ada Runner", "a:tf", 0))
        .await;
    let error = match result {
        Ok(_) => anyhow::bail!("malformed athlete row unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(error.message.contains("invalid ID"));
    response.assert_async().await;
    Ok(())
}

#[tokio::test]
async fn empty_athlete_name_is_an_incomplete_search() -> Result<()> {
    let mut server = Server::new_async().await;
    let response = server
        .mock("POST", "/")
        .with_header("content-type", "application/json")
        .with_body(
            json!({"d":{"count":1,"results":r#"<tr><td><a href="/athlete/1/track-and-field"></a></td></tr>"#,"pager":""}})
                .to_string(),
        )
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Ada Runner", "a:tf", 0))
        .await;
    let error = match result {
        Ok(_) => anyhow::bail!("empty athlete name unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(error.message.contains("empty name"));
    response.assert_async().await;
    Ok(())
}

#[tokio::test]
async fn duplicate_paginated_result_is_an_incomplete_search() -> Result<()> {
    let mut server = Server::new_async().await;
    let first = server
        .mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"start":0})))
        .with_header("content-type", "application/json")
        .with_body(
            json!({"d":{"count":2,"results":row(1),"pager":"<li data-start='1'>Next</li>"}})
                .to_string(),
        )
        .create_async()
        .await;
    let second = server
        .mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"start":1})))
        .with_header("content-type", "application/json")
        .with_body(json!({"d":{"count":2,"results":row(1),"pager":""}}).to_string())
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Ada Runner", "a:tf", 0))
        .await;
    let error = match result {
        Ok(_) => anyhow::bail!("duplicate paginated result unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(error.message.contains("duplicate"));
    first.assert_async().await;
    second.assert_async().await;
    Ok(())
}

#[tokio::test]
async fn empty_paginated_page_is_an_incomplete_search() -> Result<()> {
    let mut server = Server::new_async().await;
    let first = server
        .mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"start":0})))
        .with_header("content-type", "application/json")
        .with_body(
            json!({"d":{"count":2,"results":row(1),"pager":"<li data-start='1'>Next</li>"}})
                .to_string(),
        )
        .create_async()
        .await;
    let second = server
        .mock("POST", "/")
        .match_body(Matcher::PartialJson(json!({"start":1})))
        .with_header("content-type", "application/json")
        .with_body(json!({"d":{"count":2,"results":"","pager":""}}).to_string())
        .create_async()
        .await;
    let result = AthleticNetClient::new(&config(&server))?
        .execute_exhaustive(&SearchRequest::new("Ada Runner", "a:tf", 0))
        .await;
    let error = match result {
        Ok(_) => anyhow::bail!("empty paginated result unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(error.message.contains("no progress"));
    first.assert_async().await;
    second.assert_async().await;
    Ok(())
}
