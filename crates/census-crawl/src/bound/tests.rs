use super::collect::emit::title_matches_slug;
use super::collect::index::{
    filter_secondary, name_tokens, parse_school_index, resolve_slugs, slugify, verified_slug, Index,
};
use super::parse::{parse, parse_gender, parse_sport_label};
use census_domain::model::{Gender, Sport};
use std::path::{Path, PathBuf};

type Fallible = Result<(), Box<dyn std::error::Error>>;

fn fixture(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bound")
        .join(name);
    Ok(std::fs::read_to_string(path)?)
}

fn captured_staff_pages() -> Result<Vec<(String, PathBuf)>, Box<dyn std::error::Error>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bound");
    let mut files: Vec<(String, PathBuf)> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().to_string(),
                entry.path(),
            )
        })
        .filter(|(name, path)| {
            name.starts_with("staff")
                && path.extension().and_then(|ext| ext.to_str()) == Some("html")
        })
        .collect();
    files.sort();
    Ok(files)
}

#[test]
fn parse_coaches_from_staff_page() -> Fallible {
    let html = fixture("staff-siouxcenter-ia-girls-xc.html")?;
    let parsed = parse(&html);
    check!(
        eq;
        parsed.page.title_school,
        "Sioux Center Warriors"
    );
    check!(eq; parsed.page.title_sport, "Girls Cross Country");
    let names: Vec<&str> = parsed.coaches.iter().map(|c| c.name.as_str()).collect();
    check!(
        eq;
        names,
        vec![
            "Brock Lehman",
            "Jennifer Vande Vegte",
            "Kyle Cleveringa",
            "Elijah Weaver"
        ]
    );
    check!(
        eq;
        parsed
            .coaches
            .iter()
            .filter(|c| c.role == "Head Coach")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn parse_coaches_from_sd_staff_page() -> Fallible {
    let html = fixture("staff-yankton-sd-girls-xc.html")?;
    let parsed = parse(&html);
    check!(
        eq;
        parsed.page.title_school,
        "Yankton Gazelles"
    );
    check!(
        eq;
        parsed.coaches.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        vec!["Mitchell Gullikson", "Evan Steiner"]
    );
    Ok(())
}

#[test]
fn parse_coaches_from_boys_page() -> Fallible {
    let html = fixture("staff-yankton-sd-boys-xc.html")?;
    let parsed = parse(&html);
    check!(
        eq;
        parsed.page.title_school,
        "Yankton Bucks"
    );
    check!(
        eq;
        parsed.coaches.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        vec!["Mitchell Gullikson", "Evan Steiner"]
    );
    Ok(())
}

#[test]
fn title_matches_resolved_slug_not_corpus_name() -> Fallible {
    check!(
        title_matches_slug("yankton", "Yankton Gazelles"),
        "a mascot title must match its resolved slug"
    );
    check!(
        title_matches_slug("siouxcenter", "Sioux Center Warriors"),
        "a club-name title must match its resolved slug"
    );
    check!(
        !title_matches_slug("holytrinity", "White River Lady Tigers"),
        "another school's page must be rejected"
    );
    check!(
        !title_matches_slug("yankton", ""),
        "an empty title must be rejected"
    );
    Ok(())
}

#[test]
fn different_school_page_yields_its_own_coaches() -> Fallible {
    let html = fixture("staff-whiteriver-sd-girls-xc.html")?;
    let parsed = parse(&html);
    check!(
        eq;
        parsed.page.title_school,
        "White River Lady Tigers"
    );
    check!(
        !title_matches_slug("holytrinity", &parsed.page.title_school),
        "the whiteriver page must not match another school's slug"
    );
    check!(
        eq;
        parsed.coaches.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        vec!["Casey Emery-Krogman", "Richard Charging Hawk"]
    );
    Ok(())
}

#[test]
fn empty_staff_page_yields_no_coaches() -> Fallible {
    let html = fixture("staff-faith-sd-boys-xc-empty-staff.html")?;
    let parsed = parse(&html);
    check!(
        eq;
        parsed.page.title_school,
        "Faith Longhorns"
    );
    check!(
        parsed.coaches.is_empty(),
        "expected no coaches from a real page whose staff table has no rows, got {}",
        parsed.coaches.len()
    );
    Ok(())
}

#[test]
fn non_coach_role_is_skipped() -> Fallible {
    let html = fixture("staff-non-coach-role.html")?;
    let parsed = parse(&html);
    check!(
        !parsed.coaches.is_empty(),
        "expected the coach row from a page that also lists a non-coach role"
    );
    check!(
        parsed.coaches.iter().all(|c| c.role == "Head Coach"
            || c.role == "Co-Head Coach"
            || c.role == "Assistant Coach"
            || c.role == "Volunteer Coach"),
        "expected only coach roles, found non-coach role in: {:?}",
        parsed.coaches
    );
    Ok(())
}

#[test]
fn every_captured_staff_page_parses() -> Fallible {
    let files = captured_staff_pages()?;
    check!(files.len() >= 6, "expected at least 6 staff page fixtures");
    for (name, path) in files {
        let document = std::fs::read_to_string(&path)?;
        let parsed = parse(&document);
        check!(
            !parsed.page.title_school.is_empty(),
            "{name} parsed to empty title school"
        );
        check!(
            parsed.coaches.is_empty() || !parsed.page.title_school.is_empty(),
            "{name} has coaches but no title school"
        );
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
fn slugify_strips_non_alphanumeric() -> Fallible {
    check!(eq; slugify("Sioux Center High School"), "siouxcenterhighschool");
    check!(eq; slugify("Yankton Gazelles"), "yanktongazelles");
    check!(eq; slugify("St. Mary's, Remsen"), "stmarysremsen");
    check!(
        eq;
        slugify("Council Bluffs, Thomas Jefferson"),
        "councilbluffsthomasjefferson"
    );
    Ok(())
}

#[test]
fn name_tokens_filters_stopwords() -> Fallible {
    let tokens = name_tokens("Sioux Center High School");
    check!(
        tokens.contains(&"SIOUX".to_string()),
        "expected SIOUX in tokens"
    );
    check!(
        tokens.contains(&"CENTER".to_string()),
        "expected CENTER in tokens"
    );
    check!(
        !tokens.contains(&"HIGH".to_string()),
        "HIGH should be filtered as stopword"
    );
    check!(
        !tokens.contains(&"SCHOOL".to_string()),
        "SCHOOL should be filtered as stopword"
    );
    Ok(())
}

#[test]
fn verified_slug_looks_up_ia_schools() -> Fallible {
    check!(
        eq;
        verified_slug("Ankeny Christian Academy"),
        Some("ankenychristian".to_string())
    );
    check!(
        eq;
        verified_slug("Interstate 35, Truro"),
        Some("i35".to_string())
    );
    check!(
        eq;
        verified_slug("West Central Valley, Stuart"),
        Some("wcvalley".to_string())
    );
    check!(eq; verified_slug("Nonexistent School"), None);
    check!(eq; verified_slug("Council Bluffs"), None);
    Ok(())
}

#[test]
fn verified_slug_handles_curly_apostrophe() -> Fallible {
    let curly = "St. Mary\u{2019}s, Remsen";
    check!(
        eq;
        verified_slug(curly),
        Some("stmaryremsen".to_string())
    );
    Ok(())
}

#[test]
fn index_parsing_extracts_school_links() -> Fallible {
    let html = r#"
        <div>
            <a href="/ia/schools/siouxcenter">Sioux Center</a>
            <a href="/ia/schools/yankton">Yankton</a>
            <a href="/ia/schools/ankenychristian">Ankeny Christian Academy</a>
            <a href="/ia/schools/dowling">Dowling Catholic</a>
        </div>
    "#;
    let index = parse_school_index(html, "ia")?;
    check!(
        eq;
        index.len(),
        4
    );
    check!(index.has_slug("siouxcenter"));
    check!(index.has_slug("ankenychristian"));
    Ok(())
}

#[test]
fn index_parsing_handles_empty() -> Fallible {
    let html = "<div></div>";
    let index = parse_school_index(html, "ia")?;
    check!(
        eq;
        index.len(),
        0
    );
    Ok(())
}

#[test]
fn resolve_slugs_slugified_match() -> Fallible {
    let mut index = Index::new();
    index.add("siouxcenter".to_string(), "Sioux Center".to_string());
    index.add("yankton".to_string(), "Yankton".to_string());

    let schools = vec![
        "Sioux Center High School".to_string(),
        "Yankton".to_string(),
    ];

    let resolved = resolve_slugs(schools, &index);
    check!(
        eq;
        resolved.matched.len(),
        2
    );
    check!(resolved
        .matched
        .iter()
        .any(|(name, slug)| name == "Sioux Center High School" && slug == "siouxcenter"));
    check!(resolved
        .matched
        .iter()
        .any(|(name, slug)| name == "Yankton" && slug == "yankton"));
    check!(
        resolved.unmatched.is_empty(),
        "expected no unmatched schools"
    );
    check!(
        resolved.ambiguous.is_empty(),
        "expected no ambiguous schools"
    );
    Ok(())
}

#[test]
fn resolve_slugs_verified_table_hit() -> Fallible {
    let mut index = Index::new();
    index.add(
        "ankenychristian".to_string(),
        "Ankeny Christian Academy".to_string(),
    );
    index.add("i35".to_string(), "Interstate 35".to_string());

    let schools = vec![
        "Ankeny Christian Academy".to_string(),
        "Interstate 35, Truro".to_string(),
    ];

    let resolved = resolve_slugs(schools, &index);
    check!(
        eq;
        resolved.matched.len(),
        2
    );
    check!(resolved
        .matched
        .iter()
        .any(|(name, slug)| name == "Ankeny Christian Academy" && slug == "ankenychristian"));
    check!(resolved
        .matched
        .iter()
        .any(|(name, slug)| name == "Interstate 35, Truro" && slug == "i35"));
    Ok(())
}

#[test]
fn resolve_slugs_ambiguous_rejected() -> Fallible {
    let mut index = Index::new();
    index.add("siouxcenter".to_string(), "Sioux Center".to_string());
    index.add("siouxcity".to_string(), "Sioux City".to_string());

    let schools = vec!["Sioux".to_string()];

    let resolved = resolve_slugs(schools, &index);
    check!(
        resolved.ambiguous.contains(&"Sioux".to_string()),
        "expected 'Sioux' to be ambiguous"
    );
    check!(resolved.matched.is_empty(), "expected no matched schools");
    Ok(())
}

#[test]
fn resolve_slugs_token_match() -> Fallible {
    let mut index = Index::new();
    index.add(
        "mansonnorthwestwebster".to_string(),
        "Manson Northwest Webster".to_string(),
    );
    index.add("siouxcenter".to_string(), "Sioux Center".to_string());

    let schools = vec![
        "Manson Northwest Webster High School".to_string(),
        "Sioux Center High School".to_string(),
    ];

    let resolved = resolve_slugs(schools, &index);
    check!(
        eq;
        resolved.matched.len(),
        2
    );
    check!(resolved.matched.iter().any(|(name, slug)| name
        == "Manson Northwest Webster High School"
        && slug == "mansonnorthwestwebster"));
    check!(resolved
        .matched
        .iter()
        .any(|(name, slug)| name == "Sioux Center High School" && slug == "siouxcenter"));
    check!(eq; resolved.ambiguous.len(), 0);
    check!(eq; resolved.unmatched.len(), 0);
    Ok(())
}

#[test]
fn filter_secondary_drops_middle_schools() -> Fallible {
    let mut index = Index::new();
    index.add("douglas".to_string(), "Douglas High".to_string());
    index.add("douglasms".to_string(), "Douglas Middle School".to_string());
    index.add("elementary".to_string(), "Douglas Elementary".to_string());

    let candidates = vec!["douglas", "douglasms", "elementary"];
    let preferred = filter_secondary(&candidates, &index);
    check!(
        eq;
        preferred.len(),
        1
    );
    check!(
        preferred[0] == "douglas",
        "expected only varsity school to pass filter"
    );
    Ok(())
}

#[test]
fn a_fetched_page_answers_the_school_observation_contract() -> Fallible {
    let facts = super::map::ProfileFacts {
        name: "Yankton High School",
        key: "sd/yankton",
        url: "https://www.gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/yankton/v/staff",
        observed_on: "2026-10-07",
        state: census_domain::UsJurisdiction::SouthDakota,
    };
    let rows = vec![super::parse::CoachRow {
        role: "Head Coach".to_string(),
        name: "Mitchell Gullikson".to_string(),
    }];

    let extract = super::map::school_entities(&facts, &rows, Sport::CrossCountry, Gender::Girls);

    let observation = census_domain::model::SourceSchoolObservation::of_school(
        &super::school_namespace(),
        &extract.school,
        "2026-10-07",
    );
    let Some(observation) = observation else {
        return Err("the emitted school carries no identity for the bound namespace".into());
    };
    check!(eq; observation.source_school_id, "sd/yankton");
    check!(eq; observation.source_row_key, facts.url);
    check!(eq; extract.coaches.len(), 1);
    check!(eq; extract.coaches[0].source_identities.len(), 1);
    Ok(())
}
