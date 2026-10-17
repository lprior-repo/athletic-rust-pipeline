use crate::milesplit::{parse_roster, RosterWitnessUnavailableV1, TeamRef};
use census_domain::affiliation_candidates::SourceSpan;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const OWNER: &str = "<link rel='canonical' href='https://wi.milesplit.com/teams/9001-synthetic/roster'>";
const HEADING: &str = "<h4>Current Roster</h4>";
const ROW_OPEN: &str = "<li class=\"athlete-row data-row\">";
const ANCHOR: &str = "<a href=\"https://wi.milesplit.com/athletes/90001-synthetic\">Synthetic Person</a>";
const FIELDS: &str = "<div class=\"column-grad-year\">2027</div><div class=\"column-gender\">F</div>";

fn team() -> TeamRef {
    TeamRef {
        id: "9001".into(), slug: "synthetic".into(),
        url: "https://wi.milesplit.com/teams/9001-synthetic".into(),
        name: "Synthetic School".into(), city_state: "Synthetic, WI".into(),
    }
}

fn header(prefix: &str, heading: &str) -> String {
    format!("{prefix}{OWNER}<section>{heading}<ul id=\"rosterDataset\">{ROW_OPEN}")
}

fn page(header: &str, row: &str) -> String {
    format!("{header}{row}</li></ul></section>")
}

#[test]
fn source_statements_map_selected_live_anchor_not_identical_inert_bytes() -> TestResult {
    let prefix = "é雪\r\n<!-- prefix -->";
    let head = header(prefix, HEADING);
    let decoys = format!("<!-- {ANCHOR} --><div hidden>{ANCHOR}</div><template>{ANCHOR}</template><script>{ANCHOR}</script><textarea>{ANCHOR}</textarea><svg>{ANCHOR}</svg><div data-value='{ANCHOR}'></div>");
    let row = format!("{decoys}{ANCHOR}{FIELDS}");
    let html = page(&head, &row);
    let parsed = parse_roster(&html, team())?;
    let member = parsed.roster().and_then(|roster| roster.athletes.first()).ok_or("member")?;
    let proof = member.current_roster().ok_or("current statement")?;
    let statements = proof.source_statements_v1()?;
    let start = head.len().checked_add(decoys.len()).ok_or("person start")?;
    let end = start.checked_add(ANCHOR.len()).ok_or("person end")?;
    check!(eq; statements.person(), SourceSpan::new(start, end)?);
    check!(eq; html.get(start..end), Some(ANCHOR));
    let row_end = head.len().checked_add(row.len()).ok_or("row end")?;
    check!(eq; statements.membership(), SourceSpan::new(head.len(), row_end)?);
    let heading = proof.heading();
    check!(eq; statements.temporal(), SourceSpan::new(heading.start, heading.end)?);
    Ok(())
}

#[test]
fn utf8_and_encoded_person_keep_original_anchor_offsets() -> TestResult {
    let head = header("雪\r\n&amp;", HEADING);
    let anchor = ANCHOR.replace("Synthetic Person", "Élodie &amp; Snow");
    let row = format!("{anchor}{FIELDS}");
    let html = page(&head, &row);
    let parsed = parse_roster(&html, team())?;
    let proof = parsed.roster().and_then(|roster| roster.athletes.first())
        .and_then(|member| member.current_roster()).ok_or("current statement")?;
    let end = head.len().checked_add(anchor.len()).ok_or("anchor end")?;
    check!(eq; proof.source_statements_v1()?.person(), SourceSpan::new(head.len(), end)?);
    Ok(())
}

#[test]
fn membership_4096_is_retained_and_4097_keeps_old_current_assertion() -> TestResult {
    let head = header("", HEADING);
    let base = format!("{ANCHOR}{FIELDS}");
    for length in [4096usize, 4097] {
        let padding = length.checked_sub(base.len()).ok_or("padding")?;
        let row = format!("{}{base}", " ".repeat(padding));
        check!(eq; row.len(), length);
        let html = page(&head, &row);
        let parsed = parse_roster(&html, team())?;
        let proof = parsed.roster().and_then(|roster| roster.athletes.first())
            .and_then(|member| member.current_roster()).ok_or("old current assertion")?;
        if length == 4096 {
            let end = head.len().checked_add(length).ok_or("row end")?;
            check!(eq; proof.source_statements_v1()?.membership(), SourceSpan::new(head.len(), end)?);
        } else {
            check!(matches!(proof.source_statements_v1(), Err(RosterWitnessUnavailableV1::SourceSpanLimit)));
        }
    }
    Ok(())
}

#[test]
fn unavailable_active_person_does_not_gain_a_statement_span() -> TestResult {
    let head = header("", HEADING);
    for row in [
        format!("<!-- {ANCHOR} -->{FIELDS}"),
        format!("<template>{ANCHOR}</template>{FIELDS}"),
        format!("<div hidden>{ANCHOR}</div>{FIELDS}"),
        format!("<script>{ANCHOR}</script>{FIELDS}"),
        format!("<svg>{ANCHOR}</svg>{FIELDS}"),
    ] {
        let html = page(&head, &row);
        let parsed = parse_roster(&html, team())?;
        let member = parsed.roster().and_then(|roster| roster.athletes.first()).ok_or("raw member")?;
        check!(member.current_roster().is_none());
    }
    Ok(())
}

#[test]
fn temporal_4096_witness_preserves_existing_4097_parser_refusal() -> TestResult {
    let base = "<h4 data-pad=''>Current Roster</h4>";
    for length in [4096usize, 4097] {
        let padding = length.checked_sub(base.len()).ok_or("padding")?;
        let heading = base.replacen("data-pad=''", &format!("data-pad='{}'", " ".repeat(padding)), 1);
        let head = header("", &heading);
        let html = page(&head, &format!("{ANCHOR}{FIELDS}"));
        let parsed = parse_roster(&html, team())?;
        let member = parsed.roster().and_then(|roster| roster.athletes.first()).ok_or("member")?;
        if length == 4096 {
            let proof = member.current_roster().ok_or("current assertion")?;
            let range = proof.heading();
            check!(eq; proof.source_statements_v1()?.temporal(), SourceSpan::new(range.start, range.end)?);
        } else {
            check!(member.current_roster().is_none());
            check!(eq; member.temporal_unavailability(), Some(crate::milesplit::RosterTemporalUnavailable::ResourceLimit));
        }
    }
    Ok(())
}
