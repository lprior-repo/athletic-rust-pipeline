use super::*;

#[test]
fn unclosed_nonvoid_inert_elements_remain_structurally_refused() -> TestResult {
    for prefix in ["<template/>", "<div hidden/>"] {
        let body = document(&format!("{prefix}{}", scoped(&member_html()?)));
        check!(matches!(
            parse_roster(&body, team()),
            Err(crate::CrawlError::Schema { .. })
        ));
    }
    Ok(())
}

#[test]
fn removing_canonical_link_keeps_independent_json_ld_owner() -> TestResult {
    let body = MASON.replace(
        "<link rel=\"canonical\" href=\"https://oh.milesplit.com/teams/10002-mason/roster\" />",
        "",
    );
    check!(ne; body.as_str(), MASON);
    check!(!body.contains("rel=\"canonical\""));
    check!(body.contains("application/ld+json"));
    let verdict = parse_roster(&body, team())?;
    let roster = verdict.roster().ok_or("physical rows")?;
    check!(!roster.athletes.is_empty());
    check!(roster
        .athletes
        .iter()
        .all(|member| member.current_roster().is_some()));
    Ok(())
}
