use super::super::{analyse, Rules, Signals};
use crate::CrawlError;
use census_domain::model::{CoachRole, Gender, Sport};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const URL: &str = "https://school.example/staff";
const CARD: &str = "<table><tr><td>Transportation</td></tr></table>";
const CONTACT: &str = "<table><tr><th>Sport</th><th>Coach</th></tr><tr><td>Cross Country - Girls</td><td>Mary Ann Lee</td></tr></table>";

fn parse(html: &str) -> Result<Signals, CrawlError> {
    let mut signals = Signals::default();
    analyse(&Rules::new()?, URL, html, &mut signals)?;
    Ok(signals)
}

#[test]
fn school_sites_tables_single_row_staff_cards_do_not_exhaust_admission() -> TestResult {
    let html = CARD.repeat(24);
    let rules = Rules::new()?;
    check!(super::discover(&rules, &html)?.is_empty());
    let signals = parse(&html)?;
    check!(signals.coach_hits.is_empty());
    check!(eq; signals.pages, vec![URL.to_owned()]);
    Ok(())
}

#[test]
fn school_sites_tables_find_contact_after_twenty_four_staff_cards() -> TestResult {
    let html = format!("{}{CONTACT}", CARD.repeat(24));
    let signals = parse(&html)?;
    check!(eq; signals.coach_hits.len(), 1);
    let hit = signals.coach_hits.first().ok_or("table coach candidate")?;
    check!(eq; hit.name.as_str(), "Mary Ann Lee");
    check!(eq; hit.sport, Sport::CrossCountry);
    check!(eq; hit.gender, Gender::Girls);
    check!(eq; hit.role, CoachRole::HeadCoach);
    check!(eq; hit.url.as_str(), URL);
    check!(hit.email.is_none());
    Ok(())
}

#[test]
fn school_sites_tables_eight_eligible_tables_pass_with_interleaved_cards() -> TestResult {
    let html = format!("{CARD}{CONTACT}{CARD}").repeat(8);
    let rules = Rules::new()?;
    check!(eq; super::discover(&rules, &html)?.len(), 8);
    check!(eq; parse(&html)?.coach_hits.len(), 8);
    Ok(())
}

#[test]
fn school_sites_tables_ninth_eligible_table_fails_before_signal_mutation() -> TestResult {
    let rules = Rules::new()?;
    let mut signals = parse(CONTACT)?;
    let before = signals.clone();
    let html = format!("{CARD}{CONTACT}{CARD}").repeat(9);
    check!(matches!(
        analyse(&rules, URL, &html, &mut signals),
        Err(CrawlError::Resource {
            resource: "school site tables",
            requested: 9,
            limit: 8,
        })
    ));
    check!(eq; signals, before);
    Ok(())
}

#[test]
fn school_sites_tables_raw_row_budget_still_counts_ineligible_tables() -> TestResult {
    check!(parse(&CARD.repeat(320))?.coach_hits.is_empty());
    check!(matches!(
        parse(&CARD.repeat(321)),
        Err(CrawlError::Resource {
            resource: "school site table rows",
            requested: 321,
            limit: 320,
        })
    ));
    Ok(())
}

#[test]
fn school_sites_tables_body_budget_is_unchanged() -> TestResult {
    let padding = crate::school_sites::MAX_PAGE_BYTES
        .checked_sub(CARD.len())
        .and_then(|remaining| remaining.checked_sub(7))
        .ok_or("body fixture padding")?;
    let mut html = format!("{CARD}<!--{}-->", "x".repeat(padding));
    check!(parse(&html)?.coach_hits.is_empty());
    html.push(' ');
    check!(matches!(
        parse(&html),
        Err(CrawlError::Resource {
            resource: "school site body bytes",
            ..
        })
    ));
    Ok(())
}

#[test]
fn school_sites_tables_keep_existing_forty_data_row_window() -> TestResult {
    let header = "<tr><th>Sport</th><th>Coach</th></tr>";
    let row = "<tr><td>Cross Country - Girls</td><td>Mary Ann Lee</td></tr>";
    let later = "<tr><td>Track</td><td>John Smith</td></tr>";
    let html = format!("<table>{header}{}{later}</table>", row.repeat(40));
    let signals = parse(&html)?;
    check!(eq; signals.coach_hits.len(), 40);
    check!(signals
        .coach_hits
        .iter()
        .all(|hit| hit.name == "Mary Ann Lee"));
    Ok(())
}

#[test]
fn school_sites_tables_unqualified_headers_still_produce_no_table_candidates() -> TestResult {
    let html = "<table><tr><th>Activity</th><th>Participant</th></tr><tr><td>Track</td><td>John Smith</td></tr></table>";
    check!(super::discover(&Rules::new()?, html)?.is_empty());
    check!(parse(html)?.coach_hits.is_empty());
    Ok(())
}
