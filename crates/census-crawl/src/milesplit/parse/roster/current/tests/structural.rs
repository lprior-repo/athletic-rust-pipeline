use super::*;

#[test]
fn unclosed_inert_elements_never_admit_current_membership() -> TestResult {
    let template = document(&format!("<template/>{}", scoped(&member_html()?)));
    check!(
        matches!(parse_roster(&template, team()), Err(crate::CrawlError::Schema { detail, .. })
        if detail == "roster HTML nesting exceeds representation")
    );
    let hidden = document(&format!("<div hidden/>{}", scoped(&member_html()?)));
    assert_unavailable(&hidden, Gap::InactiveMember)?;
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
