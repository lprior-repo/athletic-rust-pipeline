use super::super::{
    apply_school_mailbox_capture, offices, CanonicalSchool, FetchOutcome, SchoolYear,
};
use super::parse;
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn first_link_budget_reason_survives_later_tokens_and_unclosed_scopes() -> TestResult {
    let body = format!("<a href='{}'>contact</a><template>", "x".repeat(4097));
    let page = parse(body.as_bytes())?;
    check!(page.limited);
    check!(eq; page.first_issue.as_deref(), Some("school contact link exceeds 4096 bytes"));
    let error = offices::mailbox_candidates(body.as_bytes())
        .err()
        .ok_or("expected failure")?;
    check!(error
        .to_string()
        .contains("school contact link exceeds 4096 bytes"));
    check!(matches!(&error, crate::CrawlError::ParserBudget { .. }));
    check!(eq; crate::coach_directories::research_failure::crawl(&error),
        census_domain::model::ContactResearchOutcome::Partial);
    check!(!error.retryable());
    Ok(())
}

#[test]
fn token_budget_has_its_own_retained_reason() -> TestResult {
    let body = "<span></span>".repeat(33_000);
    let page = parse(body.as_bytes())?;
    check!(page.limited);
    check!(eq; page.first_issue.as_deref(), Some("school contact page exceeds 65,536 HTML tokens"));
    Ok(())
}

#[test]
fn unclosed_inert_scope_has_a_specific_reason() -> TestResult {
    let page = parse(b"<template>")?;
    check!(page.malformed);
    check!(eq; page.first_issue.as_deref(), Some("school contact page ends with an unclosed inert scope"));
    Ok(())
}

#[test]
fn earlier_tokenizer_error_is_not_replaced_by_later_budget() -> TestResult {
    let first = parse(b"<div a='x' a='y'></div>")?;
    check!(first.malformed);
    let body = format!(
        "<div a='x' a='y'></div><a href='{}'>contact</a>",
        "x".repeat(4097)
    );
    let page = parse(body.as_bytes())?;
    check!(page.limited);
    check!(eq; page.first_issue, first.first_issue);
    let error = offices::mailbox_candidates(body.as_bytes())
        .err()
        .ok_or("expected malformed refusal")?;
    check!(matches!(error, crate::CrawlError::Schema { .. }));
    Ok(())
}

#[test]
fn canonical_attempt_retains_exact_parser_reason_and_source() -> TestResult {
    let body = format!(
        "<meta property='og:site_name' content='Example School'><a href='{}'>contact</a>",
        "x".repeat(4097)
    )
    .into_bytes();
    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Michigan, "Example School", "example", None);
    school.school_website = Some("https://school.example/".to_owned());
    let capture = FetchOutcome {
        url: "https://school.example/contact".to_owned(),
        response_url: Some("https://school.example/staff".to_owned()),
        method: "GET".to_owned(),
        status: 200,
        content_digest: crate::net::cache::content_digest(&body),
        bytes: body.len(),
        fetched_at: "2026-10-07T12:34:56Z".to_owned(),
        from_cache: true,
        content_type: Some("text/html".to_owned()),
        body,
    };
    apply_school_mailbox_capture(&mut school, SchoolYear::new(2026).ok_or("year")?, &capture)?;
    let attempts: Vec<_> = school
        .contact_research
        .iter()
        .flat_map(|row| &row.attempts)
        .collect();
    check!(!attempts.is_empty());
    for attempt in attempts {
        check!(eq; attempt.reason.as_str(), "school contact link exceeds 4096 bytes");
        check!(eq; attempt.locator.as_str(), "https://school.example/staff");
        check!(eq; attempt.source_sha256.as_deref(), Some(capture.content_digest.as_str()));
        check!(eq; attempt.acquired_at.as_str(), capture.fetched_at.as_str());
    }
    Ok(())
}

#[test]
fn every_school_page_wrapper_keeps_budget_classification() -> TestResult {
    let body = format!("<a href='{}'>contact</a>", "x".repeat(4097));
    let rules = crate::school_sites::Rules::new()?;
    let errors = [
        super::super::inspect_staff_links(body.as_bytes()).err(),
        super::super::school_coach_blocks(body.as_bytes()).err(),
        offices::mailbox_candidates(body.as_bytes()).err(),
        super::super::profiles::candidates(&rules, "https://school.example/staff", body.as_bytes())
            .err(),
    ];
    for error in errors {
        let error = error.ok_or("expected bounded page refusal")?;
        check!(matches!(&error, crate::CrawlError::ParserBudget { .. }));
        check!(error
            .to_string()
            .contains("school contact link exceeds 4096 bytes"));
        check!(eq; crate::coach_directories::research_failure::crawl(&error),
            census_domain::model::ContactResearchOutcome::Partial);
    }
    Ok(())
}
