use super::super::{published_owner, requested_owner};
use super::*;
use crate::milesplit::wire::RosterOwnerWitnessPolicy;
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

fn read(html: &str, policy: RosterOwnerWitnessPolicy) -> CrawlResult<Published> {
    let team = team();
    published_owner(html, &team, requested_owner(&team)?, policy)
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
        expected.push((kind, start, html.len()));
    }
    let start = html.len();
    html.push_str(&json);
    expected.extend([
        (Kind::JsonLdUrl, start, html.len()),
        (Kind::JsonLdId, start, html.len()),
    ]);
    let published = read(&html, RosterOwnerWitnessPolicy::CurrentBody)?;
    check!(published.found);
    let witness = published.witness?;
    check!(eq; witness.declarations().len(), expected.len());
    for (actual, (kind, from, to)) in witness.declarations().iter().zip(expected) {
        check!(eq; actual.provider_id(), 9001);
        check!(eq; actual.jurisdiction(), Some(UsJurisdiction::from_code("WI").ok_or("state")?));
        check!(eq; actual.kind(), kind);
        check!(eq; actual.span().range(), from..to);
    }
    Ok(())
}

#[test]
fn historical_policy_4096_is_retained_and_4097_only_withholds_new_witness() -> TestResult {
    let base = format!("<link data-pad='' rel='canonical' href='{URL}'>");
    for length in [4096usize, 4097] {
        let padding = length.checked_sub(base.len()).ok_or("padding")?;
        let html = base.replacen(
            "data-pad=''",
            &format!("data-pad='{}'", " ".repeat(padding)),
            1,
        );
        check!(eq; html.len(), length);
        let published = read(&html, RosterOwnerWitnessPolicy::Historical4096)?;
        check!(published.found);
        if length == 4096 {
            let witness = published.witness?;
            check!(eq; witness.declarations().first().ok_or("owner")?.span().range(), 0..4096);
        } else {
            check!(matches!(published.witness, Err(Gap::SourceSpanLimit)));
        }
    }
    Ok(())
}

#[test]
fn current_policy_retains_4097_byte_owner() -> TestResult {
    let base = format!("<link data-pad='' rel='canonical' href='{URL}'>");
    let length = 4097usize;
    let padding = length.checked_sub(base.len()).ok_or("padding")?;
    let html = base.replacen(
        "data-pad=''",
        &format!("data-pad='{}'", " ".repeat(padding)),
        1,
    );
    check!(eq; html.len(), length);
    let published = read(&html, RosterOwnerWitnessPolicy::CurrentBody)?;
    check!(published.found);
    let witness = published.witness?;
    check!(eq; witness.declarations().first().ok_or("owner")?.span().range(), 0..4097);
    Ok(())
}

#[test]
fn current_policy_large_jsonld_retains_closed_and_eof_raw_spans() -> TestResult {
    let prefix = "é雪\r\n";
    let padding = " ".repeat(50000);
    for (end, termination) in [
        ("</script>", Termination::ScriptEnd),
        ("", Termination::ScriptEof),
    ] {
        let html = format!(
            "{prefix}<script type='application/ld+json'>{{\"@type\":\"WebPage\",\"padding\":\"{padding}\",\"url\":\"{URL}\"}}{end}"
        );
        let published = read(&html, RosterOwnerWitnessPolicy::CurrentBody)?;
        check!(published.found);
        let witness = published.witness?;
        let owner = witness.declarations().first().ok_or("owner")?;
        check!(eq; owner.span().range(), prefix.len()..html.len());
        check!(eq; owner.termination(), termination);
    }
    Ok(())
}

#[test]
fn inventory_limit_never_hides_late_contrary_declarations() -> TestResult {
    let declaration = format!("<link rel='canonical' href='{URL}'>");
    let exact = read(
        &declaration.repeat(32),
        RosterOwnerWitnessPolicy::CurrentBody,
    )?
    .witness?;
    check!(eq; exact.declarations().len(), 32);
    let overflow = declaration.repeat(33);
    let read_overflow = read(&overflow, RosterOwnerWitnessPolicy::CurrentBody)?;
    check!(read_overflow.found);
    check!(matches!(
        read_overflow.witness,
        Err(Gap::OwnerInventoryLimit)
    ));
    for contrary in [
        "<meta property='og:url' content='https://wi.milesplit.com/teams/9002/roster'>",
        "<script type='application/ld+json'>{\"@type\":\"School\",\"@id\":\"https://wi.milesplit.com/teams/9002/roster\"}</script>",
    ] {
        check!(matches!(
            read(&format!("{overflow}{contrary}"), RosterOwnerWitnessPolicy::CurrentBody),
            Err(crate::CrawlError::Schema { url, .. }) if url == team().url
        ));
    }
    Ok(())
}

#[test]
fn historical_span_failure_precedes_inventory_but_never_hides_contrary_owner() -> TestResult {
    let declaration = format!("<link rel='canonical' href='{URL}'>");
    let oversized = format!(
        "<link data-pad='{}' rel='canonical' href='{URL}'>",
        " ".repeat(4096)
    );
    let html = oversized + &declaration.repeat(32);
    for (policy, expected) in [
        (
            RosterOwnerWitnessPolicy::Historical4096,
            Gap::SourceSpanLimit,
        ),
        (
            RosterOwnerWitnessPolicy::CurrentBody,
            Gap::OwnerInventoryLimit,
        ),
    ] {
        let published = read(&html, policy)?;
        check!(published.found);
        check!(eq; published.witness, Err(expected));
        let contrary = format!(
            "{html}<meta property='og:url' content='https://wi.milesplit.com/teams/9002/roster'>"
        );
        check!(matches!(
            read(&contrary, policy),
            Err(crate::CrawlError::Schema { url, .. }) if url == team().url
        ));
    }
    Ok(())
}

#[test]
fn absent_or_nested_metadata_is_not_upgraded_to_published_owner() -> TestResult {
    for html in [
        "<a href='/teams/9001/roster'>Synthetic</a>",
        "<script type='application/ld+json'>{\"@type\":\"ItemList\",\"itemListElement\":{\"@type\":\"School\",\"url\":\"/teams/9001/roster\"}}</script>",
    ] {
        let published = read(html, RosterOwnerWitnessPolicy::CurrentBody)?;
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
    let witness = read(&html, RosterOwnerWitnessPolicy::CurrentBody)?.witness?;
    let owner = witness.declarations().first().ok_or("owner")?;
    check!(eq; owner.termination(), Termination::ScriptEof);
    check!(eq; owner.span().range(), prefix.len()..html.len());
    Ok(())
}
