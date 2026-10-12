use super::*;
use crate::net::representation::{validate_user_agent, MAX_USER_AGENT_BYTES, USER_AGENT_NAME};
use crate::net::RepresentationHeaders;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const TEAMS: &str = "https://example.com/teams";
const COMPACT: &str = "Mozilla/5.0 Chrome/126.0 census-service/0.1";
const COMMENT_FORM: &str =
    "census-service/0.1 (independent HS track & field research collector; polite; contact: repo owner)";

fn client() -> TestResult<reqwest::Client> {
    Ok(reqwest::Client::builder().build()?)
}

#[test]
fn the_dispatch_site_refuses_a_user_agent_outside_the_declared_policy() -> TestResult {
    let client = client()?;
    let refused = build_request(
        &client,
        "GET",
        TEAMS,
        None,
        &[(USER_AGENT_NAME.to_string(), COMMENT_FORM.to_string())],
        None,
        false,
    );
    match refused {
        Err(FetchError::Policy { detail }) => {
            check!(
                detail.contains("parenthesized comment form"),
                "the refusal states the compact agent rule: {detail}"
            );
            check!(
                detail.contains("independent HS track"),
                "the refusal names the offending value: {detail}"
            );
        }
        Err(other) => {
            return Err(format!("the dispatch site refused for the wrong reason: {other}").into())
        }
        Ok(_) => return Err("a comment-form User-Agent reached dispatch".into()),
    }
    Ok(())
}

#[test]
fn the_dispatch_site_matches_the_user_agent_name_without_case_or_padding() -> TestResult {
    let client = client()?;
    let refused = build_request(
        &client,
        "GET",
        TEAMS,
        None,
        &[(" User-Agent ".to_string(), COMMENT_FORM.to_string())],
        None,
        false,
    );
    match refused {
        Err(FetchError::Policy { .. }) => (),
        Err(other) => {
            return Err(format!("the dispatch site refused for the wrong reason: {other}").into())
        }
        Ok(_) => return Err("a padded User-Agent name bypassed the policy".into()),
    }
    Ok(())
}

#[test]
fn a_compact_user_agent_is_carried_onto_the_built_request() -> TestResult {
    let client = client()?;
    let request = build_request(
        &client,
        "GET",
        TEAMS,
        None,
        &[
            ("Accept".to_string(), "application/json".to_string()),
            (USER_AGENT_NAME.to_string(), COMPACT.to_string()),
        ],
        None,
        false,
    )?
    .build()?;
    let declared = request
        .headers()
        .get(USER_AGENT_NAME)
        .ok_or("the declared User-Agent is absent from the built request")?;
    check!(eq; declared.to_str()?, COMPACT);
    check!(eq;
        request.headers().get("accept").and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    Ok(())
}

#[test]
fn both_enforcement_points_share_one_validator() -> TestResult {
    let over = "v".repeat(MAX_USER_AGENT_BYTES + 1);
    for value in [
        "",
        " ",
        " Mozilla/5.0",
        "Mozilla/5.0 ",
        COMMENT_FORM,
        "Mozilla/5.0\r\nAccept: text/html",
        "M\u{f6}bius/5.0",
        over.as_str(),
    ] {
        check!(
            validate_user_agent(value).is_err(),
            "the validator refuses {value:?}"
        );
        check!(
            matches!(
                RepresentationHeaders::canonical(&[(
                    USER_AGENT_NAME.to_string(),
                    value.to_string()
                )]),
                Err(FetchError::Policy { .. })
            ),
            "the representation allowlist refuses {value:?}"
        );
    }
    check!(
        validate_user_agent(COMPACT).is_ok(),
        "the validator allows a compact agent"
    );
    check!(
        validate_user_agent(&"v".repeat(MAX_USER_AGENT_BYTES)).is_ok(),
        "the validator allows an agent at the byte ceiling"
    );
    Ok(())
}
