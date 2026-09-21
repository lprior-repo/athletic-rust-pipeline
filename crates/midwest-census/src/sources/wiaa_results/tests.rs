use super::*;
use crate::model::{CompetitionLevel, GradYear, Grade};

const ARCHIVE_HTML: &str = r#"
        <h3><strong>2025 Track &amp; Field State Results</strong></h3>
        <ul><li>Division 1 - <a href="/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm">Boys</a>
        | <a href="https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1girlsstateresults.htm">Girls</a></li></ul>
        <h3><strong>2025 Track &amp; Field Sectional Results</strong></h3>
        <ul><li>Neenah - <a href="/Portals/0/PDF/Results/Track/2025/neenahsectionalb.pdf">Boys</a></li></ul>
        <a href="/sports/boys-track-field/boys-track-field">Sport home</a>
    "#;

#[test]
fn archive_links_carry_year_stem_label_and_extension() {
    let artifacts = archive_artifacts(ARCHIVE_HTML).expect("the archive page parses");
    assert_eq!(
        artifacts.len(),
        3,
        "only result-file links are artifacts: {artifacts:?}"
    );
    let first = &artifacts[0];
    assert_eq!(first.year, 2025);
    assert_eq!(first.stem, "d1boysstateresults");
    assert_eq!(first.extension, "htm");
    assert_eq!(first.label, "Boys");
    assert!(
        artifacts
            .iter()
            .all(|artifact| artifact.url.starts_with("https://www.wiaawi.org/")),
        "relative hrefs are resolved against the archive host"
    );
    assert!(artifacts.iter().any(|artifact| artifact.extension == "pdf"));
}

#[test]
fn formats_are_decided_by_extension_and_sniffed_body() {
    assert_eq!(artifact_format("pdf", None), ArtifactFormat::Pdf);
    assert_eq!(artifact_format("rtf", None), ArtifactFormat::Unparsed);
    assert_eq!(artifact_format("txt", None), ArtifactFormat::HytekText);
    assert_eq!(
        artifact_format("htm", Some("<p>HY-TEK's Meet Manager</p>")),
        ArtifactFormat::HytekHtml
    );
    assert_eq!(
        artifact_format("htm", Some("<title>RaceDay Scoring</title>")),
        ArtifactFormat::RaceDay
    );
}

#[test]
fn current_season_files_are_found_under_the_dated_upload_path() {
    // The state meet pages link the newest releases as `/sites/default/files/<year>-<month>/…`,
    // which carries no `/Results/` segment; missing them costs the whole current season.
    let body = r#"
        <a href="/sites/default/files/2026-08/trb2026d1stateresults.pdf">Boys</a>
        <a href="/sites/default/files/2026-08/tr2026arrowheadregionalindiv.pdf">Arrowhead</a>
        <a href="/sites/default/files/2026-08/tr2026beaverdamsectionalindiv.pdf">Beaver Dam</a>
        <a href="/sites/default/files/2026-08/somethingelse.pdf">Unrelated upload</a>
        <a href="/sites/default/files/2025-11/xcstate.htm">XC state</a>
    "#;
    let artifacts = archive_artifacts(body).expect("the archive page parses");
    assert_eq!(
        artifacts.len(),
        5,
        "every dated upload is treated as a candidate result file: {artifacts:?}"
    );
    assert!(artifacts
        .iter()
        .all(|artifact| artifact.year == 2026 || artifact.year == 2025));
    let state = artifacts
        .iter()
        .find(|artifact| artifact.stem == "trb2026d1stateresults")
        .expect("the 2026 state file is discovered");
    assert_eq!(state.year, 2026);
    assert_eq!(state.extension, "pdf");
    assert_eq!(state.label, "Boys");
}

#[test]
fn school_year_follows_the_sport_boundary() {
    // Spring 2025 track is inside school year 2024-25: grade 11 there is class of 2026.
    let spring = school_year_for("2025-06-06", Sport::OutdoorTrack, 2025);
    assert_eq!(spring.start_year(), 2024);
    assert_eq!(GradYear::of(Grade::new(11).unwrap(), spring).0, 2026);
    // Fall 2025 cross country opens school year 2025-26: grade 11 there is class of 2027.
    let fall = school_year_for("2025-10-25", Sport::CrossCountry, 2025);
    assert_eq!(fall.start_year(), 2025);
    assert_eq!(GradYear::of(Grade::new(11).unwrap(), fall).0, 2027);
    // A year-only date (RaceDay) still lands in the right school year per sport.
    assert_eq!(
        school_year_for("2023", Sport::CrossCountry, 2023).start_year(),
        2023
    );
    assert_eq!(
        school_year_for("2023", Sport::OutdoorTrack, 2023).start_year(),
        2022
    );
}

#[test]
fn meet_levels_come_from_the_published_name() {
    assert_eq!(
        level_of("WIAA Track & Field State Championships"),
        CompetitionLevel::State
    );
    assert_eq!(
        level_of("WIAA D2 XC Sectionals - Boys Race"),
        CompetitionLevel::Sectional
    );
    assert_eq!(level_of("Arrowhead Regional"), CompetitionLevel::Regional);
    assert_eq!(
        level_of("Some Invitational"),
        CompetitionLevel::Invitational
    );
    assert_eq!(level_of("Dual Meet"), CompetitionLevel::Unknown);
}
