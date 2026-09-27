use crate::common;

use std::collections::BTreeMap;

use anyhow::{bail, ensure, Context, Result};
use census_crawl::ohsaa;
use census_crawl::wiaa;

use super::constants;
use super::fixtures::Corpus;

pub fn wiaa_directory(corpus: &mut Corpus) -> Result<()> {
    let files = common::fixtures("wiaa")?;
    let letter = files
        .iter()
        .find(|path| {
            common::file_name(path)
                .map(|name| name.starts_with("directory_"))
                .unwrap_or(false)
        })
        .context("the wiaa corpus carries no directory letter")?;
    let index =
        wiaa::parse_directory_letter(&common::fixture("wiaa", &common::file_name(letter)?)?);
    ensure!(
        !index.is_empty(),
        "{}: the directory letter yielded no rows",
        letter.display()
    );
    let mut pages = 0usize;
    for path in &files {
        if path == letter {
            continue;
        }
        let name = common::file_name(path)?;
        let Some(org_id) = name
            .strip_prefix("school_org")
            .and_then(|rest| rest.split('_').next())
        else {
            bail!("{name} is not a known wiaa fixture: the letter or a school page");
        };
        let page = wiaa::parse_school_page(&common::fixture("wiaa", &name)?);
        let entry = index
            .iter()
            .find(|entry| entry.org_id == org_id)
            .cloned()
            .unwrap_or_else(|| wiaa::IndexEntry {
                org_id: org_id.to_string(),
                ..wiaa::IndexEntry::default()
            });
        let extract = wiaa::school_entities(&entry, &page, constants::OBSERVED_ON)
            .with_context(|| format!("{name}: the page yielded no school"))?;
        ensure!(
            !extract.coaches.is_empty(),
            "{name}: the page yielded no coach"
        );
        corpus.schools.push(extract.school);
        corpus.coaches.extend(extract.coaches);
        pages = pages.saturating_add(1);
    }
    ensure!(
        pages >= 3,
        "the wiaa corpus carries only {pages} school pages"
    );
    Ok(())
}

pub fn ohsaa_schools(corpus: &mut Corpus) -> Result<()> {
    let mut searches: BTreeMap<String, Vec<ohsaa::SearchResult>> = BTreeMap::new();
    let mut sports: BTreeMap<String, String> = BTreeMap::new();
    let mut ads: BTreeMap<String, String> = BTreeMap::new();
    for path in common::fixtures("ohsaa")? {
        let name = common::file_name(&path)?;
        let body = common::fixture("ohsaa", &name)?;
        if name.starts_with("search_") {
            searches.insert(name, ohsaa::parse_search(&body));
        } else if name.starts_with("sports_") {
            sports.insert(name, body);
        } else if name.starts_with("ad_") {
            ads.insert(name, body);
        } else {
            bail!("{name} is not a known ohsaa fixture");
        }
    }
    ensure!(
        searches.len() == 3 && sports.len() == 3 && ads.len() == 3,
        "the ohsaa corpus is nine fixtures: three searches, three sports pages, three AD pages"
    );

    let dublin = &searches["search_dublin_coffman.html"];
    ensure!(
        dublin.len() == 1,
        "the Dublin Coffman search fixture yields {} rows",
        dublin.len()
    );
    let dublin_sports = &sports["sports_dublin_coffman.html"];
    let header = ohsaa_school_from_page(dublin_sports)?;
    ensure!(
        dublin[0].name == header.name && dublin[0].ohsaa_id == header.ohsaa_id,
        "the search row {} ({}) and the school page header {} ({}) disagree",
        dublin[0].name,
        dublin[0].ohsaa_id,
        header.name,
        header.ohsaa_id
    );
    ensure!(
        ohsaa::parse_sports_table(dublin_sports)
            .iter()
            .find(|(label, ..)| label == "Cross Country")
            .is_some_and(|(_, boys, _)| boys.is_some()),
        "the Dublin Coffman sports fixture no longer publishes a boys cross-country coach"
    );
    let dublin_ad = ohsaa::parse_ad_page(&ads["ad_dublin_coffman.html"]);
    let (director, _) = dublin_ad
        .director
        .clone()
        .context("the Dublin Coffman AD fixture publishes no director")?;
    ensure!(
        !dublin_ad.office_roles.is_empty(),
        "the Dublin Coffman AD fixture publishes no office role, so nothing proves they stay out \
         of the coach list"
    );
    let extract = ohsaa::school_entities(
        &dublin[0],
        dublin_sports,
        &ads["ad_dublin_coffman.html"],
        constants::OBSERVED_ON,
    );
    ensure!(
        !extract.coaches.is_empty(),
        "the Dublin Coffman pair yields no coach"
    );
    let expected_director = ohsaa::strip_honorific(&director);
    ensure!(
        extract
            .coaches
            .iter()
            .any(|coach| coach.name == expected_director),
        "the Dublin Coffman director {expected_director:?} is missing from the coach rows"
    );
    for (_, role_holder) in &dublin_ad.office_roles {
        let role_holder = ohsaa::strip_honorific(role_holder);
        ensure!(
            !extract
                .coaches
                .iter()
                .any(|coach| coach.name == role_holder),
            "the office role {role_holder:?} became a coach row"
        );
    }
    corpus.schools.push(extract.school);
    corpus.coaches.extend(extract.coaches);

    let mentor = &searches["search_duplicate_rows.html"];
    ensure!(
        mentor.len() == 1 && !mentor[0].name.is_empty() && !mentor[0].ohsaa_id.is_empty(),
        "the duplicate-row search fixture no longer collapses to one school"
    );
    ensure!(
        ohsaa::parse_sports_table(&sports["sports_malformed.html"]).is_empty(),
        "the malformed sports fixture stopped parsing as empty"
    );
    ensure!(
        ohsaa::parse_ad_page(&ads["ad_malformed.html"])
            .director
            .is_none(),
        "the malformed AD fixture stopped parsing as empty"
    );
    let extract = ohsaa::school_entities(
        &mentor[0],
        &sports["sports_malformed.html"],
        &ads["ad_malformed.html"],
        constants::OBSERVED_ON,
    );
    ensure!(
        extract.coaches.is_empty(),
        "a school with malformed sport and AD pages yields {} coaches",
        extract.coaches.len()
    );
    corpus.schools.push(extract.school);

    let centerville_sports = &sports["sports_centerville.html"];
    let track = ohsaa::parse_sports_table(centerville_sports);
    let track = track
        .iter()
        .find(|(label, ..)| label == "Track & Field")
        .context("the Centerville sports fixture no longer publishes a Track & Field row")?;
    ensure!(
        !(track.1.is_none() && track.2.is_none()),
        "the Centerville Track & Field row names no coach at all"
    );
    let result = ohsaa_school_from_page(centerville_sports)?;
    let ad_header = ohsaa_school_from_page(&ads["ad_centerville.html"])?;
    ensure!(
        result.name == ad_header.name && result.ohsaa_id == ad_header.ohsaa_id,
        "the Centerville sports and AD fixtures name different schools: {} ({}) against {} ({})",
        result.name,
        result.ohsaa_id,
        ad_header.name,
        ad_header.ohsaa_id
    );
    let extract = ohsaa::school_entities(
        &result,
        centerville_sports,
        &ads["ad_centerville.html"],
        constants::OBSERVED_ON,
    );
    ensure!(
        !extract.coaches.is_empty(),
        "the Centerville pair yields no coach"
    );
    corpus.schools.push(extract.school);
    corpus.coaches.extend(extract.coaches);

    ensure!(
        searches["search_no_results.html"].is_empty(),
        "the empty search fixture no longer parses as empty"
    );
    Ok(())
}

fn ohsaa_school_from_page(html: &str) -> Result<ohsaa::SearchResult> {
    let header = html
        .split_once("<h2>")
        .and_then(|(_, rest)| rest.split_once("</h2>"))
        .map(|(header, _)| header.trim())
        .context("the page carries no <h2> school header")?;
    let (name, id) = header
        .split_once(" (")
        .with_context(|| format!("the school header {header:?} carries no OHSAA id"))?;
    let id = id.strip_suffix(')').unwrap_or(id).trim();
    ensure!(
        !name.is_empty() && !id.is_empty(),
        "the school header {header:?} is incomplete"
    );
    let city = html
        .split_once(", OH ")
        .and_then(|(head, _)| head.lines().last())
        .map(str::trim)
        .filter(|city| !city.is_empty())
        .context("the page carries no city on its address line")?;
    Ok(ohsaa::SearchResult {
        name: name.to_string(),
        city: city.to_string(),
        ohsaa_id: id.to_string(),
    })
}
