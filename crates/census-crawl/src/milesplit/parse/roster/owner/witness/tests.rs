use super::super::{published_owner, requested_owner};
use super::*;
use census_domain::affiliation_candidates::SourceSpan;
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const URL: &str = "https://wi.milesplit.com/teams/9001-synthetic/roster";

fn team() -> TeamRef {
    TeamRef {
        id: "9001".into(),
        slug: "synthetic".into(),
        url: "https://wi.milesplit.com/teams/9001-synthetic".into(),
        name: "Synthetic School".into(),
        city_state: "Synthetic, WI".into(),
    }
}

fn read(html: &str) -> CrawlResult<Published> {
    let team = team();
    published_owner(html, &team, requested_owner(&team)?)
}

#[test]
fn all_canonical_meta_and_both_json_fields_remain_in_one_complete_inventory() -> TestResult {
    let prefix = "é雪\r\n";
    let parts = [
        (
            format!("<link rel='canon&#105;cal' href='{URL}'>"),
            Kind::Canonical,
        ),
        (
            format!("<meta property='og&#58;url' content='{URL}'>"),
            Kind::OpenGraph,
        ),
        (
            format!("<meta name='twitter:url' content='{URL}'>"),
            Kind::Twitter,
        ),
    ];
    let json = format!("<script type='application/ld+json'>{{\"@type\":\"WebPage\",\"url\":\"{URL}\",\"@id\":\"{URL}\"}}</script>");
    let mut html = prefix.to_owned();
    let mut expected = Vec::new();
    for (part, kind) in parts {
        let start = html.len();
        html.push_str(&part);
        expected.push((kind, SourceSpan::new(start, html.len())?));
    }
    let start = html.len();
    html.push_str(&json);
    let span = SourceSpan::new(start, html.len())?;
    expected.extend([(Kind::JsonLdUrl, span), (Kind::JsonLdId, span)]);
    let published = read(&html)?;
    check!(published.found);
    let witness = published.witness?;
    check!(eq; witness.declarations().len(), expected.len());
    for (actual, (kind, span)) in witness.declarations().iter().zip(expected) {
        check!(eq; actual.provider_id(), 9001);
        check!(eq; actual.jurisdiction(), Some(UsJurisdiction::from_code("WI").ok_or("state")?));
        check!(eq; actual.kind(), kind);
        check!(eq; actual.span(), span);
    }
    Ok(())
}

#[test]
fn owner_statement_4096_is_retained_and_4097_only_withholds_new_witness() -> TestResult {
    let base = format!("<link data-pad='' rel='canonical' href='{URL}'>");
    for length in [4096usize, 4097] {
        let padding = length.checked_sub(base.len()).ok_or("padding")?;
        let html = base.replacen(
            "data-pad=''",
            &format!("data-pad='{}'", " ".repeat(padding)),
            1,
        );
        check!(eq; html.len(), length);
        let published = read(&html)?;
        check!(published.found);
        if length == 4096 {
            let witness = published.witness?;
            check!(eq; witness.declarations().first().ok_or("owner")?.span(), SourceSpan::new(0, 4096)?);
        } else {
            check!(matches!(published.witness, Err(Gap::SourceSpanLimit)));
        }
    }
    Ok(())
}

#[test]
fn inventory_limit_never_hides_late_contrary_declarations() -> TestResult {
    let declaration = format!("<link rel='canonical' href='{URL}'>");
    let exact = read(&declaration.repeat(32))?.witness?;
    check!(eq; exact.declarations().len(), 32);
    let overflow = declaration.repeat(33);
    let read_overflow = read(&overflow)?;
    check!(read_overflow.found);
    check!(matches!(
        read_overflow.witness,
        Err(Gap::OwnerInventoryLimit)
    ));
    for contrary in [
        "<meta property='og:url' content='https://wi.milesplit.com/teams/9002/roster'>",
        "<script type='application/ld+json'>{\"@type\":\"School\",\"@id\":\"https://wi.milesplit.com/teams/9002/roster\"}</script>",
    ] {
        check!(read(&format!("{overflow}{contrary}")).is_err());
    }
    Ok(())
}

#[test]
fn absent_or_nested_metadata_is_not_upgraded_to_published_owner() -> TestResult {
    for html in [
        "<a href='/teams/9001/roster'>Synthetic</a>",
        "<script type='application/ld+json'>{\"@type\":\"ItemList\",\"itemListElement\":{\"@type\":\"School\",\"url\":\"/teams/9001/roster\"}}</script>",
    ] {
        let published = read(html)?;
        check!(!published.found);
        check!(matches!(published.witness, Err(Gap::MissingPublishedOwner)));
    }
    Ok(())
}

#[test]
fn json_ld_eof_retains_existing_semantics_with_exact_end() -> TestResult {
    let prefix = "é雪\r\n";
    let html = format!(
        "{prefix}<script type='application/ld+json'>{{\"@type\":\"WebPage\",\"url\":\"{URL}\"}}"
    );
    let witness = read(&html)?.witness?;
    let owner = witness.declarations().first().ok_or("owner")?;
    check!(eq; owner.termination(), Termination::ScriptEof);
    check!(eq; owner.span(), SourceSpan::new(prefix.len(), html.len())?);
    Ok(())
}
