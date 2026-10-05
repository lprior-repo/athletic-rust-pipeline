use super::parse::{
    parse_coach_rows, parse_gender, parse_school_info, parse_search_json, parse_sport_label,
};
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
