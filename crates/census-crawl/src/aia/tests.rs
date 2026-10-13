use super::map::qualify_owner;
use super::parse::{
    parse_coach_rows, parse_gender, parse_school_info, parse_school_profile, parse_search_json,
    parse_sport_label, ParsedRow,
};
use crate::net::FetchOutcome;
use census_domain::model::{Gender, Sport};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/aia")
        .join(name)
}

fn html_fixtures() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/aia");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("html")
        })
        .collect();
    files.sort();
    files
}

#[test]
fn parses_search_json_fixture() -> anyhow::Result<()> {
    let json = std::fs::read_to_string(fixture("search_chandler.json"))?;
    let rows = parse_search_json(&json)?;
    anyhow::ensure!(!rows.is_empty(), "search JSON parsed to no rows");
    anyhow::ensure!(
        rows.iter().any(|row| row.name == "Chandler"),
        "expected Chandler school in search results"
    );
    Ok(())
}

#[test]
fn parses_school_profiles() -> anyhow::Result<()> {
    let chandler = std::fs::read_to_string(fixture("school_100_chandler.html"))?;
    let profile = parse_school_info(&chandler)?;
    anyhow::ensure!(
        profile.name == "Chandler High School",
        "unexpected school name: {}",
        profile.name
    );
    anyhow::ensure!(profile.address.is_some(), "address missing for Chandler");
    Ok(())
}

#[test]
fn parses_coach_rows() -> anyhow::Result<()> {
    let chandler = std::fs::read_to_string(fixture("school_100_chandler.html"))?;
    let rows = parse_coach_rows(&chandler);
    anyhow::ensure!(!rows.is_empty(), "no coach rows parsed");

    let has_boys_cross_country = rows.iter().any(|row| {
        parse_sport_label(&row.sport_label) == Some(Sport::CrossCountry)
            && parse_gender(&row.sport_label) == Gender::Boys
    });
    anyhow::ensure!(has_boys_cross_country, "no boys cross country coach found");

    let has_outdoor_track = rows
        .iter()
        .any(|row| parse_sport_label(&row.sport_label) == Some(Sport::OutdoorTrack));
    anyhow::ensure!(has_outdoor_track, "no track coach found");

    Ok(())
}

#[test]
fn parses_all_html_fixtures() -> anyhow::Result<()> {
    let files = html_fixtures();
    anyhow::ensure!(!files.is_empty(), "no HTML fixtures captured");
    for file in files {
        let document = std::fs::read_to_string(&file)?;
        let rows = parse_coach_rows(&document);
        anyhow::ensure!(
            !rows.is_empty(),
            "{} parsed to no coach rows",
            file.display()
        );
    }
    Ok(())
}

#[test]
fn sport_label_parsing() -> anyhow::Result<()> {
    anyhow::ensure!(
        parse_sport_label("Cross Country - Boy's") == Some(Sport::CrossCountry),
        "boys cross country label not parsed"
    );
    anyhow::ensure!(
        parse_sport_label("Track - Boy's") == Some(Sport::OutdoorTrack),
        "live track label not parsed"
    );
    anyhow::ensure!(
        parse_sport_label("Track & Field - Girl's") == Some(Sport::OutdoorTrack),
        "girls track label not parsed"
    );
    anyhow::ensure!(
        parse_sport_label("Football").is_none(),
        "non-relevant sport should not parse"
    );
    Ok(())
}

#[test]
fn gender_parsing() -> anyhow::Result<()> {
    anyhow::ensure!(
        parse_gender("Cross Country - Boy's") == Gender::Boys,
        "boys gender not detected"
    );
    anyhow::ensure!(
        parse_gender("Cross Country - Girl's") == Gender::Girls,
        "girls gender not detected"
    );
    anyhow::ensure!(
        parse_gender("Polo") == Gender::Mixed,
        "mixed gender not detected"
    );
    Ok(())
}

fn profile_capture(url: &str, response_url: Option<&str>, body: &str) -> FetchOutcome {
    FetchOutcome {
        url: url.to_string(),
        response_url: response_url.map(str::to_string),
        method: "GET".to_string(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body.as_bytes()),
        bytes: body.len(),
        fetched_at: "2026-10-04T00:00:00Z".to_string(),
        from_cache: false,
        content_type: Some("text/html".to_string()),
        body: body.as_bytes().to_vec(),
    }
}

#[test]
fn chandler_indexed_owner_refuses_hamilton_profile() -> anyhow::Result<()> {
    let chandler_body = std::fs::read_to_string(fixture("school_100_chandler.html"))?;
    let hamilton_body = std::fs::read_to_string(fixture("school_116_hamilton.html"))?;
    let chandler_profile = parse_school_profile(&chandler_body)?;
    let hamilton_profile = parse_school_profile(&hamilton_body)?;
    anyhow::ensure!(
        chandler_profile.info.name == "Chandler High School",
        "unexpected Chandler owner: {}",
        chandler_profile.info.name
    );
    anyhow::ensure!(
        hamilton_profile.info.name == "Hamilton High School",
        "unexpected Hamilton owner: {}",
        hamilton_profile.info.name
    );
    let indexed = ParsedRow {
        school_id: "100".to_string(),
        name: "Chandler".to_string(),
        full_name: Some("Chandler High School".to_string()),
        city: Some("Chandler".to_string()),
    };
    let matched_url = "https://aiaonline.org/schools/100";
    let matched = profile_capture(matched_url, None, &chandler_body);
    anyhow::ensure!(
        qualify_owner(&indexed, &chandler_profile, &matched).is_ok(),
        "matching Chandler owner must qualify"
    );
    let mismatched = profile_capture(matched_url, None, &hamilton_body);
    let detail = match qualify_owner(&indexed, &hamilton_profile, &mismatched) {
        Ok(()) => anyhow::bail!("Hamilton profile must not qualify as Chandler appointment"),
        Err(crate::CrawlError::Schema { detail, .. }) => detail,
        Err(error) => anyhow::bail!("wrong owner must be typed Schema, got: {error}"),
    };
    anyhow::ensure!(
        detail.contains("matching published school owner"),
        "unexpected wrong-owner detail: {detail}"
    );
    let redirected = profile_capture(
        matched_url,
        Some("https://aiaonline.org/schools/116"),
        &chandler_body,
    );
    let redirect_detail = match qualify_owner(&indexed, &chandler_profile, &redirected) {
        Ok(()) => anyhow::bail!("Hamilton redirect must not qualify as Chandler appointment"),
        Err(crate::CrawlError::Schema { detail, .. }) => detail,
        Err(error) => anyhow::bail!("wrong redirect must be typed Schema, got: {error}"),
    };
    anyhow::ensure!(
        redirect_detail.contains("redirect differs"),
        "unexpected redirect detail: {redirect_detail}"
    );
    Ok(())
}
