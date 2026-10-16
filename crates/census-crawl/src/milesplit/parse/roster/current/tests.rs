mod structural;

use super::*;
use crate::milesplit::parse_roster;
use census_domain::model::Sport;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const MASON: &str =
    include_str!("../../../../../tests/fixtures/milesplit/oh_roster_10002_mason.html");

fn team() -> TeamRef {
    TeamRef {
        id: "10002".into(),
        slug: "mason".into(),
        url: "https://oh.milesplit.com/teams/10002-mason".into(),
        name: "Mason".into(),
        city_state: "Mason, OH".into(),
    }
}

#[test]
fn retained_mason_heading_proves_only_scoped_member_observation() -> TestResult {
    let verdict = parse_roster(MASON, team())?;
    let roster = verdict.roster().ok_or("retained roster")?;
    check!(!roster.athletes.is_empty());
    for member in &roster.athletes {
        let proof = member.current_roster().ok_or("scoped statement")?;
        check!(eq; proof.owner(), "10002");
        check!(eq; proof.member(), member.athlete().athlete_id);
        check!(eq; proof.row(), member.locator());
        let heading = MASON.get(proof.heading()).ok_or("heading span")?;
        check!(eq; heading, "<h4>Current Roster</h4>");
        check!(eq; proof.heading_sha256(), &<[u8; 32]>::from(Sha256::digest(heading.as_bytes())));
        check!(proof.section().start < proof.heading().start);
        check!(proof.container().start < proof.row().byte_offset);
        check!(eq; proof.supports(Sport::CrossCountry), member.athlete().xc);
    }
    Ok(())
}

#[test]
fn flags_and_graduation_year_without_current_statement_remain_undated() -> TestResult {
    for replacement in [
        "<h4>Roster</h4>",
        "<h4>2027 Roster</h4>",
        "<h4>Past Roster</h4>",
        "",
    ] {
        let body = MASON.replace("<h4>Current Roster</h4>", replacement);
        let verdict = parse_roster(&body, team())?;
        let roster = verdict.roster().ok_or("valid undated rows")?;
        check!(!roster.athletes.is_empty());
        check!(roster
            .athletes
            .iter()
            .all(|member| member.current_roster().is_none()));
    }
    Ok(())
}

#[test]
fn unrelated_inert_or_different_section_heading_cannot_date_members() -> TestResult {
    for replacement in [
        "<!-- <h4>Current Roster</h4> -->",
        "<script>\"<h4>Current Roster</h4>\"</script>",
        "<template><h4>Current Roster</h4></template>",
        "<noscript><h4>Current Roster</h4></noscript>",
        "<div title=\"<h4>Current Roster</h4>\"></div>",
        "<svg><h4>Current Roster</h4></svg>",
        "<div><h4>Current Roster</h4></div>",
        "<h4 hidden>Current Roster</h4>",
        "<section><h4>Current Roster</h4></section>",
        "<h4>Current Roster</h4><h4>Former Members</h4>",
    ] {
        let body = MASON.replace("<h4>Current Roster</h4>", replacement);
        let verdict = parse_roster(&body, team())?;
        let roster = verdict.roster().ok_or("valid undated rows")?;
        check!(roster
            .athletes
            .iter()
            .all(|member| member.current_roster().is_none()));
    }
    Ok(())
}

#[test]
fn tag_shaped_member_bytes_in_comment_do_not_receive_statement_authority() -> TestResult {
    let body = MASON.replacen(
        "<li class=\"athlete-row data-row\">",
        "<!-- <li class=\"athlete-row data-row\">",
        1,
    );
    let start = body
        .find("<!-- <li class=\"athlete-row data-row\">")
        .ok_or("member")?;
    let end = body
        .get(start..)
        .and_then(|tail| tail.find("</li>"))
        .ok_or("member end")?
        + start
        + "</li>".len();
    let mut body = body;
    body.insert_str(end, " -->");
    let verdict = parse_roster(&body, team())?;
    let roster = verdict.roster().ok_or("physical rows")?;
    check!(roster
        .athletes
        .first()
        .ok_or("first row")?
        .current_roster()
        .is_none());
    check!(roster
        .athletes
        .iter()
        .skip(1)
        .any(|member| member.current_roster().is_some()));
    Ok(())
}

#[test]
fn current_statement_without_verified_document_owner_remains_untrusted() -> TestResult {
    let body = scoped(&member_html()?);
    check!(!super::super::owner::validate_published(&body, &team())?);
    let verdict = parse_roster(&body, team())?;
    let roster = verdict.roster().ok_or("physical rows")?;
    check!(!roster.athletes.is_empty());
    check!(roster
        .athletes
        .iter()
        .all(|member| member.current_roster().is_none()));
    Ok(())
}

fn member_html() -> Result<String, Box<dyn std::error::Error>> {
    let parsed = parse_roster(MASON, team())?;
    let member = parsed
        .roster()
        .and_then(|roster| roster.athletes.first())
        .ok_or("member")?;
    let row = member.locator();
    let end = row.byte_offset.checked_add(row.byte_length).ok_or("span")?;
    Ok(MASON
        .get(row.byte_offset..end)
        .ok_or("member bytes")?
        .to_string())
}

fn document(content: &str) -> String {
    format!("<link rel=\"canonical\" href=\"https://oh.milesplit.com/teams/10002-mason/roster\">{content}")
}

fn scoped(content: &str) -> String {
    format!("<section><h4>Current Roster</h4><ul id=\"rosterDataset\"><li class=\"athlete-row data-row\">{content}</li></ul></section>")
}

fn assert_unavailable(body: &str, expected: Gap) -> TestResult {
    let parsed = parse_roster(body, team())?;
    let roster = parsed.roster().ok_or("retained physical member")?;
    check!(!roster.athletes.is_empty());
    for member in &roster.athletes {
        check!(member.current_roster().is_none());
        check!(eq; member.temporal_unavailability(), Some(expected));
    }
    Ok(())
}

#[test]
fn actual_member_container_cannot_date_comment_template_or_hidden_fields() -> TestResult {
    let member = member_html()?;
    for body in [
        format!("<!--{member}-->"),
        format!("<template>{member}</template>"),
        format!("<div hidden>{member}</div>"),
    ] {
        assert_unavailable(&document(&scoped(&body)), Gap::InactiveMember)?;
    }
    Ok(())
}

#[test]
fn duplicate_heading_or_dataset_container_is_ambiguous() -> TestResult {
    let body = scoped(&member_html()?);
    let duplicate_heading = body.replace(
        "<h4>Current Roster</h4>",
        "<h4>Current Roster</h4><h4>Current Roster</h4>",
    );
    assert_unavailable(&document(&duplicate_heading), Gap::AmbiguousStatement)?;
    let list_start = body.find("<ul").ok_or("list")?;
    let list_end = body
        .find("</ul>")
        .ok_or("list end")?
        .checked_add(5)
        .ok_or("span")?;
    let list = body.get(list_start..list_end).ok_or("list span")?;
    let duplicate_container = body.replace("</section>", &format!("{list}</section>"));
    assert_unavailable(&document(&duplicate_container), Gap::AmbiguousStatement)?;
    Ok(())
}

#[test]
fn nonvoid_self_closing_syntax_does_not_end_inert_html_elements() -> TestResult {
    for (prefix, close) in [("<template/>", "</template>"), ("<div hidden/>", "</div>")] {
        let body = document(&format!("{prefix}{}{close}", scoped(&member_html()?)));
        assert_unavailable(&body, Gap::InactiveMember)?;
    }
    Ok(())
}

#[test]
fn malformed_attributes_after_scope_creation_invalidate_temporal_authority() -> TestResult {
    let body = scoped(&member_html()?);
    for malformed in [
        body.replace("<ul id=", "<ul data-x=\"a\" data-x=\"b\" id="),
        body.replace("<li class=", "<li data-x=\"a\" data-x=\"b\" class="),
    ] {
        let malformed = malformed.replace(
            "<li data-x=\"a\" data-x=\"b\" class=\"athlete-row data-row\">",
            "<li class=\"athlete-row data-row\"><div data-x=\"a\" data-x=\"b\"></div>",
        );
        assert_unavailable(&document(&malformed), Gap::MalformedMarkup)?;
    }
    Ok(())
}

#[test]
fn escaped_text_and_quoted_attributes_cannot_reconstitute_live_member_markup() -> TestResult {
    let member = member_html()?;
    let quoted = format!("<div title='{member}'></div>");
    assert_unavailable(&document(&scoped(&quoted)), Gap::InactiveMember)?;
    let escaped = member
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let body = format!("<!--{member}-->{escaped}");
    assert_unavailable(&document(&scoped(&body)), Gap::InactiveMember)?;
    Ok(())
}

#[test]
fn heading_raw_bytes_are_bounded_and_hashed_once_per_scope() -> TestResult {
    let body = scoped(&member_html()?);
    let oversized = body.replace(
        "Current Roster</h4>",
        &format!(
            "Current Roster<!--{}--></h4>",
            "x".repeat(MAX_HEADING_BYTES)
        ),
    );
    assert_unavailable(&document(&oversized), Gap::ResourceLimit)?;
    let scan = scan(MASON).map_err(|_| "retained scan")?;
    check!(scan.rows.len() > scan.heading_hashes);
    check!(eq; scan.heading_hashes, 1);
    Ok(())
}

#[test]
fn section_closure_updates_only_its_index_and_depth_remains_bounded() -> TestResult {
    let body = document(&format!(
        "{}{}",
        scoped(&member_html()?),
        "<section></section>".repeat(1000)
    ));
    let parsed = scan(&body).map_err(|_| "bounded scan")?;
    check!(eq; parsed.section_updates, 1001);
    check!(eq; parsed.scopes.len(), 1001);
    check!(eq; parsed.heading_hashes, 1);
    let deep = document(&format!(
        "{}{}",
        "<div>".repeat(MAX_DEPTH),
        scoped(&member_html()?)
    ));
    assert_unavailable(&deep, Gap::ResourceLimit)?;
    Ok(())
}

#[test]
fn unsupported_tree_repair_contexts_cannot_own_current_roster_scope() -> TestResult {
    for wrapper in ["select", "table", "tbody", "tr", "td", "frameset"] {
        let body = document(&format!(
            "<{wrapper}>{}</{wrapper}>",
            scoped(&member_html()?)
        ));
        assert_unavailable(&body, Gap::InactiveMember)?;
    }
    Ok(())
}
