use crate::common;

use std::collections::BTreeMap;

use anyhow::{bail, ensure, Context, Result};
use census_crawl::ohsaa;
use census_crawl::wiaa;

use super::constants;
use super::fixtures::Corpus;

fn seeded_capture(url: &str, body: &str) -> census_crawl::net::FetchOutcome {
    use sha2::{Digest, Sha256};
    census_crawl::net::FetchOutcome {
        url: url.to_string(),
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: format!("{:x}", Sha256::digest(body.as_bytes())),
        bytes: body.len(),
        fetched_at: format!("{}T00:00:00Z", constants::OBSERVED_ON),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body: body.as_bytes().to_vec(),
    }
}

pub fn wiaa_directory(corpus: &mut Corpus) -> Result<()> {
    let files = common::fixtures("wiaa")?;
    let letter = files
        .iter()
        .find(|path| {
            common::file_name(path)
                .map(|name| name.starts_with("directory_"))
                .map_or(false, |value| value)
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
        let entry = match index.iter().find(|entry| entry.org_id == org_id).cloned() {
            Some(value) => value,
            None => wiaa::IndexEntry {
                org_id: org_id.to_string(),
                ..wiaa::IndexEntry::default()
            },
        };
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

    ohsaa_dublin(
        corpus,
        &searches["search_dublin_coffman.html"],
        &sports["sports_dublin_coffman.html"],
        &ads["ad_dublin_coffman.html"],
    )?;
    ohsaa_malformed(
        corpus,
        &searches["search_duplicate_rows.html"],
        &sports["sports_malformed.html"],
        &ads["ad_malformed.html"],
    )?;
    ohsaa_centerville(
        corpus,
        &sports["sports_centerville.html"],
        &ads["ad_centerville.html"],
    )?;
    ensure!(
        searches["search_no_results.html"].is_empty(),
        "empty OHSAA search yielded a school"
    );
    Ok(())
}

fn ohsaa_dublin(
    corpus: &mut Corpus,
    rows: &[ohsaa::SearchResult],
    sports: &str,
    ad: &str,
) -> Result<()> {
    ensure!(rows.len() == 1, "Dublin Coffman search must yield one row");
    let result = rows.first().context("Dublin Coffman search row")?;
    let header = ohsaa_school_from_page(sports)?;
    ensure!(
        result.name == header.name && result.ohsaa_id == header.ohsaa_id,
        "Dublin Coffman search and sports owners disagree"
    );
    ensure!(
        ohsaa::parse_sports_table(sports)
            .iter()
            .find(|(label, ..)| label == "Cross Country")
            .is_some_and(|(_, boys, _)| boys.is_some()),
        "Dublin Coffman must publish a boys cross-country coach"
    );
    let page = ohsaa::parse_ad_page(ad);
    let extract = ohsaa::school_entities(
        result,
        &seeded_capture(&result.sports_url(), sports),
        Some(&seeded_capture(&result.ad_url(), ad)),
    );
    ensure!(
        !extract.coaches.is_empty(),
        "Dublin Coffman pair yielded no coaches"
    );
    ohsaa_office_roles(&page, &extract.coaches)?;
    corpus.schools.push(extract.school);
    corpus.coaches.extend(extract.coaches);
    Ok(())
}

fn ohsaa_office_roles(
    page: &ohsaa::AdPage,
    coaches: &[census_domain::model::CanonicalCoach],
) -> Result<()> {
    let (director, _) = page.director.as_ref().context("Dublin Coffman director")?;
    ensure!(
        !page.office_roles.is_empty(),
        "Dublin Coffman must publish office roles"
    );
    let expected_director = ohsaa::strip_honorific(director);
    ensure!(
        coaches.iter().any(|coach| coach.name == expected_director
            && coach.role == census_domain::model::CoachRole::AthleticDirector),
        "Dublin Coffman director {expected_director:?} missing"
    );
    page.office_roles.iter().try_for_each(|(_, holder)| {
        let holder = ohsaa::strip_honorific(holder);
        ensure!(
            !coaches.iter().any(|coach| coach.name == holder),
            "office holder {holder:?} became a coach"
        );
        Ok::<_, anyhow::Error>(())
    })
}

fn ohsaa_malformed(
    corpus: &mut Corpus,
    rows: &[ohsaa::SearchResult],
    sports: &str,
    ad: &str,
) -> Result<()> {
    ensure!(
        rows.len() == 1,
        "duplicate OHSAA rows must collapse to one school"
    );
    let result = rows.first().context("deduplicated OHSAA school")?;
    ensure!(
        !result.name.is_empty() && !result.ohsaa_id.is_empty(),
        "school owner incomplete"
    );
    ensure!(
        ohsaa::parse_sports_table(sports).is_empty(),
        "malformed sports yielded rows"
    );
    ensure!(
        ohsaa::parse_ad_page(ad).director.is_none(),
        "malformed AD yielded a director"
    );
    let extract = ohsaa::school_entities(
        result,
        &seeded_capture(&result.sports_url(), sports),
        Some(&seeded_capture(&result.ad_url(), ad)),
    );
    ensure!(
        extract.coaches.is_empty(),
        "malformed OHSAA pages fabricated coaches"
    );
    corpus.schools.push(extract.school);
    Ok(())
}

fn ohsaa_centerville(corpus: &mut Corpus, sports: &str, ad: &str) -> Result<()> {
    let sections = ohsaa::parse_sports_table(sports);
    let track = sections
        .iter()
        .find(|(label, ..)| label == "Track & Field")
        .context("Centerville Track & Field row")?;
    ensure!(
        track.1.is_some() || track.2.is_some(),
        "Centerville track row has no coach"
    );
    let result = ohsaa_school_from_page(sports)?;
    let ad_header = ohsaa_school_from_page(ad)?;
    ensure!(
        result.name == ad_header.name && result.ohsaa_id == ad_header.ohsaa_id,
        "Centerville sports and AD owners disagree"
    );
    let extract = ohsaa::school_entities(
        &result,
        &seeded_capture(&result.sports_url(), sports),
        Some(&seeded_capture(&result.ad_url(), ad)),
    );
    ensure!(
        !extract.coaches.is_empty(),
        "Centerville pair yielded no coaches"
    );
    ensure!(
        extract
            .coaches
            .iter()
            .all(|coach| coach.school == extract.school_id && coach.source_identities.is_empty()),
        "Centerville people must link to the school without invented source-person IDs"
    );
    corpus.schools.push(extract.school);
    corpus.coaches.extend(extract.coaches);
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
    let id = id.strip_suffix(')').map_or(id, |value| value).trim();
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
