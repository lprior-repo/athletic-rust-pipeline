use super::parse::{
    parse_coaches_from_profile, parse_directory_links, parse_gender, parse_school_profile,
    parse_sport_label,
};
use census_domain::model::Gender;
use census_domain::model::Sport;
use std::path::{Path, PathBuf};

type Fallible = Result<(), Box<dyn std::error::Error>>;

fn fixture(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/uhsaa")
        .join(name);
    Ok(std::fs::read_to_string(path)?)
}

fn captured_profiles() -> Result<Vec<(String, PathBuf)>, Box<dyn std::error::Error>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/uhsaa");
    let mut files: Vec<(String, PathBuf)> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().to_string(),
                entry.path(),
            )
        })
        .filter(|(name, path)| {
            name.starts_with("profile")
                && path.extension().and_then(|ext| ext.to_str()) == Some("html")
        })
        .collect();
    files.sort();
    Ok(files)
}

#[test]
fn directory_capture_lists_member_schools() -> Fallible {
    let links = parse_directory_links(&fixture("directory.html")?);
    check!(
        links.len() >= 162,
        "directory capture parsed {} member links",
        links.len()
    );
    let mut urls: Vec<&str> = links.iter().map(|link| link.url.as_str()).collect();
    let listed = urls.len();
    urls.sort_unstable();
    urls.dedup();
    check!(eq; urls.len(), listed, "a member URL was listed twice");
    let alta = links
        .iter()
        .find(|link| link.url.contains("id=Alta&Reg=6&schoolID=1"))
        .ok_or("the directory capture does not list Alta")?;
    check!(eq; alta.name, "Alta Hawks");
    Ok(())
}

#[test]
fn directory_links_accept_both_quote_styles() -> Fallible {
    let html = r#"<a href='../school-directory/?id=Alta&Reg=6&schoolID=1' title='Alta Hawks'>
<a href="../school-directory/?id=Murray&Reg=10&schoolID=73" title="Murray Spartans">
<a href="../school-directory/?id=Alta&Reg=6&schoolID=1" title="Alta Hawks"><img src="Alta.png"></a>"#;
    let links = parse_directory_links(html);
    check!(eq; links.len(), 2);
    check!(eq; links[0].name, "Alta Hawks");
    check!(eq; links[1].name, "Murray Spartans");
    Ok(())
}

#[test]
fn profile_captures_publish_xc_and_track_coaches() -> Fallible {
    let expected: [(&str, [(&str, &str); 4]); 3] = [
        (
            "profile-alta.html",
            [
                ("Boys Cross Country", "Rebecca Bennion"),
                ("Girls Cross Country", "Rebecca Bennion"),
                ("Boys Track & Field", "Rebecca Bennion"),
                ("Girls Track & Field", "Rebecca Bennion"),
            ],
        ),
        (
            "profile-murray.html",
            [
                ("Boys Cross Country", "Diana Stewart"),
                ("Girls Cross Country", "Diana Stewart"),
                ("Boys Track & Field", "JennaBree Tollestrup"),
                ("Girls Track & Field", "JennaBree Tollestrup"),
            ],
        ),
        (
            "profile-herriman.html",
            [
                ("Boys Cross Country", "Josh Pugel"),
                ("Girls Cross Country", "Josh Pugel"),
                ("Boys Track & Field", "Corey Wales"),
                ("Girls Track & Field", "Corey Wales"),
            ],
        ),
    ];

    for (file, rows_expected) in expected {
        let rows = parse_coaches_from_profile(&fixture(file)?);
        let observed: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| (row.sport_label.as_str(), row.name.as_str()))
            .collect();
        for (sport, coach) in rows_expected {
            check!(
                observed.contains(&(sport, coach)),
                "{file} does not publish {coach} for {sport}: {observed:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn profile_capture_reads_school_details() -> Fallible {
    let profile = parse_school_profile(&fixture("profile-alta.html")?);
    check!(eq; profile.name, "Alta Hawks");
    check!(eq; profile.district, "Canyons");
    check!(eq; profile.classification, "5A");
    check!(eq; profile.region, "6");
    check!(
        profile.address.starts_with("11055 S. 1000 E."),
        "{}",
        profile.address
    );
    check!(profile.coaches.len() >= 4, "{}", profile.coaches.len());
    let bennion = profile
        .coaches
        .iter()
        .find(|row| row.sport_label == "Boys Cross Country")
        .ok_or("no boys cross country row")?;
    check!(eq; bennion.email, "Rebecca.Bennion@canyonsdistrict.org");
    Ok(())
}

#[test]
fn season_header_rows_are_not_coach_rows() -> Fallible {
    let html = r#"<table>
<tr class="sport-season"><td colspan="4" style="background-color: #000000;">Fall Sports</td></tr>
<tr><td><i class="fas fa-running"></i> Boys Cross Country</td>
<td><a href="mailto:coach@example.org"> Coach Name</a></td><td></td></tr>
<tr><td></td><td>()</td><td></td></tr>
</table>"#;
    let rows = parse_coaches_from_profile(html);
    check!(eq; rows.len(), 1);
    check!(eq; rows[0].sport_label, "Boys Cross Country");
    check!(eq; rows[0].name, "Coach Name");
    check!(eq; rows[0].email, "coach@example.org");
    Ok(())
}

#[test]
fn every_captured_profile_parses() -> Fallible {
    let files = captured_profiles()?;
    check!(files.len() >= 3, "expected at least 3 profile captures");
    for (name, path) in files {
        let document = std::fs::read_to_string(&path)?;
        let rows = parse_coaches_from_profile(&document);
        check!(!rows.is_empty(), "{name} parsed to no coach rows");
    }
    Ok(())
}

#[test]
fn parse_sport_label_correctly() -> Fallible {
    check!(
        eq;
        parse_sport_label("Boys Cross Country"),
        Some(Sport::CrossCountry)
    );
    check!(
        eq;
        parse_sport_label("Girls Cross Country"),
        Some(Sport::CrossCountry)
    );
    check!(
        eq;
        parse_sport_label("Boys Track & Field"),
        Some(Sport::OutdoorTrack)
    );
    check!(
        eq;
        parse_sport_label("Girls Track & Field"),
        Some(Sport::OutdoorTrack)
    );
    check!(eq; parse_sport_label("Football"), None);
    Ok(())
}

#[test]
fn parse_gender_correctly() -> Fallible {
    check!(eq; parse_gender("Boys Cross Country"), Gender::Boys);
    check!(eq; parse_gender("Girls Cross Country"), Gender::Girls);
    Ok(())
}

#[test]
fn american_fork_space_href_encodes_without_whitespace_and_preserves_owner() -> Fallible {
    let raw_absolute = "https://uhsaa.org/school-directory/?id=American Fork&Reg=3&schoolID=3";
    let raw_relative = "../school-directory/?id=American Fork&Reg=3&schoolID=3";
    for raw in [raw_absolute, raw_relative] {
        let encoded = super::owner::request_url(raw)
            .map_err(|error| format!("request_url failed for {raw:?}: {error}"))?;
        check!(
            !encoded.chars().any(char::is_whitespace),
            "encoded request retains whitespace: {encoded:?}"
        );
        check!(
            url::Url::parse(&encoded).is_ok(),
            "encoded request is not a URL: {encoded:?}"
        );
        let raw_id =
            super::owner::school_id(raw).map_err(|error| format!("raw owner failed: {error}"))?;
        let encoded_id = super::owner::school_id(&encoded)
            .map_err(|error| format!("encoded owner failed: {error}"))?;
        check!(eq; encoded_id, raw_id);
        check!(eq; encoded_id.as_str(), "3");
    }
    let from_absolute =
        super::owner::request_url(raw_absolute).map_err(|error| error.to_string())?;
    let from_relative =
        super::owner::request_url(raw_relative).map_err(|error| error.to_string())?;
    check!(eq; from_absolute, from_relative);
    let parsed = url::Url::parse(&from_absolute).map_err(|error| error.to_string())?;
    let id_param = parsed
        .query_pairs()
        .find(|(key, _)| key == "id")
        .map(|(_, value)| value.into_owned())
        .ok_or("id query missing")?;
    check!(eq; id_param, "American Fork");
    Ok(())
}

#[test]
fn american_fork_html_ampersand_decodes_to_same_owner() -> Fallible {
    let html = r#"<a href="../school-directory/?id=American Fork&amp;Reg=3&amp;schoolID=3" title="American Fork Cavemen">x</a>"#;
    let published = super::published::read(html).map_err(|error| error.to_string())?;
    check!(eq; published.links.len(), 1);
    let href = published.links[0].url.clone();
    check!(
        !href.contains("&amp;"),
        "href retains HTML entity: {href:?}"
    );
    let id = super::owner::school_id(&href).map_err(|error| error.to_string())?;
    check!(eq; id.as_str(), "3");
    let encoded = super::owner::request_url(&href).map_err(|error| error.to_string())?;
    check!(
        !encoded.chars().any(char::is_whitespace),
        "encoded request retains whitespace: {encoded:?}"
    );
    check!(eq; super::owner::school_id(&encoded).map_err(|error| error.to_string())?, id);
    Ok(())
}

#[test]
fn foreign_and_downgrade_profile_hrefs_stay_denied() -> Fallible {
    for raw in [
        "http://uhsaa.org/school-directory/?id=Alta&Reg=6&schoolID=1",
        "https://foreign.invalid/school-directory/?id=Alta&Reg=6&schoolID=1",
        "https://uhsaa.org/school-directory/?id=Alta&Reg=6",
        "https://uhsaa.org/school-directory/?id=Alta&Reg=6&schoolID=",
    ] {
        check!(
            super::owner::request_url(raw).is_err(),
            "request_url accepted {raw:?}"
        );
        check!(
            super::owner::school_id(raw).is_err(),
            "school_id accepted {raw:?}"
        );
    }
    Ok(())
}
