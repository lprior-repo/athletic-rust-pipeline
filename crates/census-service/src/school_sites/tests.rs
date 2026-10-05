use super::artifacts;
use super::contacts::{contact_rows, csv_sport, real_person};
use super::queue::{self, PlannedSite, QueueStats};
use census_crawl::school_sites::{
    queue_key, AdHit, CoachHit, PageEvidence, QueueRow, Signals, SiteOutcome,
};
use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn hit(name: &str, sport: Sport, email: Option<&str>) -> CoachHit {
    CoachHit {
        name: name.to_string(),
        sport,
        context: format!("{name} Cross Country Coach"),
        url: "https://example.k12.us/coaches".to_string(),
        title: "Coaches".to_string(),
        role: CoachRole::HeadCoach,
        gender: Gender::Girls,
        email: email.map(str::to_string),
    }
}

fn site(state: UsJurisdiction, school: &str, website: &str) -> PlannedSite {
    let key = queue_key(state.code(), school);
    PlannedSite {
        state,
        school: school.to_string(),
        website: website.to_string(),
        row: QueueRow {
            state: state.code().to_string(),
            name: school.to_string(),
            website: website.to_string(),
        },
        artifact: PathBuf::from(format!("{key}.json")),
    }
}

#[test]
fn person_cleaning_keeps_real_names_only() -> TestResult {
    check!(eq; real_person("Jane Doe", "Rock Bridge High School").as_deref(), Some("Jane Doe"));
    check!(eq; real_person("Coach John Q. Public", "Rock Bridge High School").as_deref(), Some("John Q. Public"));
    check!(eq; real_person("Athletic Director", "Rock Bridge High School"), None);
    check!(eq; real_person("Rock Bridge", "Rock Bridge High School"), None);
    check!(eq; real_person("Jane", "Rock Bridge High School"), None);
    check!(eq; real_person("Wildcats Track", "Rock Bridge High School"), None);
    Ok(())
}

#[test]
fn contact_rows_carry_page_provenance() -> TestResult {
    let signals = Signals {
        emails: vec!["mlsmith@example.k12.us".to_string()],
        coach_hits: vec![hit("Mary Smith", Sport::CrossCountry, None)],
        ad_hits: vec![AdHit {
            name: "Robert Jones".to_string(),
            context: "Athletic Director Robert Jones".to_string(),
            url: "https://example.k12.us/athletics".to_string(),
            title: "Athletics".to_string(),
        }],
        pages: vec![
            "https://example.k12.us/coaches".to_string(),
            "https://example.k12.us/athletics".to_string(),
        ],
    };
    let outcome = SiteOutcome {
        signals,
        requests: 2,
        errors: 0,
        note: None,
        pages: vec![
            PageEvidence {
                url: "https://example.k12.us/coaches".to_string(),
                digest: "def456".to_string(),
                fetched_at: "2026-10-05T04:00:00Z".to_string(),
                status: 200,
            },
            PageEvidence {
                url: "https://example.k12.us/athletics".to_string(),
                digest: "abc123".to_string(),
                fetched_at: "2026-10-05T05:00:00Z".to_string(),
                status: 200,
            },
        ],
    };
    let planned = site(
        UsJurisdiction::Missouri,
        "Rock Bridge High School",
        "https://example.k12.us",
    );
    let rows = contact_rows(&planned, &outcome);
    check!(eq; rows.len(), 2);
    let coach = rows.iter().find(|row| !row.coach_name.is_empty());
    let coach = coach.ok_or("no coach row")?;
    check!(eq; coach.coach_name.as_str(), "Mary Smith");
    check!(eq; coach.sport.as_str(), "Cross Country");
    check!(eq; coach.role.as_str(), "Head Coach");
    check!(eq; coach.public_professional_email.as_str(), "mlsmith@example.k12.us");
    check!(eq; coach.last_observed.as_str(), "2026-10-05");
    check!(eq; coach.state.as_str(), "MO");
    let director = rows.iter().find(|row| !row.ad_name.is_empty());
    let director = director.ok_or("no director row")?;
    check!(eq; director.role.as_str(), "Athletic Director");
    check!(eq; director.sport.as_str(), "");
    check!(eq; director.ad_email.as_str(), "");
    check!(eq; director.source_url.as_str(), "https://example.k12.us/athletics");
    check!(eq; csv_sport(Sport::OutdoorTrack), "Track");
    Ok(())
}

#[test]
fn queue_planning_filters_and_names_artifacts() -> TestResult {
    let rows = vec![
        QueueRow {
            state: "MO".to_string(),
            name: "Rock Bridge High School".to_string(),
            website: " example.k12.us ".to_string(),
        },
        QueueRow {
            state: String::new(),
            name: "Hickman High School".to_string(),
            website: "hickman.k12.us".to_string(),
        },
        QueueRow {
            state: "MO".to_string(),
            name: "Rock Bridge High School".to_string(),
            website: "example.k12.us".to_string(),
        },
        QueueRow {
            state: "MO".to_string(),
            name: "No Website High School".to_string(),
            website: String::new(),
        },
        QueueRow {
            state: "California".to_string(),
            name: "Oakland Tech".to_string(),
            website: "oaklandtech.example.org".to_string(),
        },
    ];
    let mut stats = QueueStats::default();
    let sites = queue::plan(
        rows,
        Some(UsJurisdiction::Missouri),
        None,
        None,
        std::path::Path::new("/tmp/school-sites"),
        &mut stats,
    );
    check!(eq; sites.len(), 3);
    check!(eq; stats.read, 5);
    check!(eq; stats.without_website, 1);
    check!(eq; stats.duplicate, 1);
    check!(eq; sites[0].website.as_str(), "https://example.k12.us");
    check!(eq; sites[1].state, UsJurisdiction::Missouri);
    check!(eq; sites[1].artifact.file_name().and_then(|name| name.to_str()), Some("MO__hickman-high-school.json"));
    check!(eq; sites[2].state, UsJurisdiction::California);
    check!(eq; sites[2].row.website.as_str(), "https://oaklandtech.example.org");
    Ok(())
}

#[test]
fn queue_sampling_is_evenly_spaced() -> TestResult {
    let rows: Vec<QueueRow> = (0..10)
        .map(|index| QueueRow {
            state: "MO".to_string(),
            name: format!("School {index}"),
            website: format!("school{index}.k12.us"),
        })
        .collect();
    let mut stats = QueueStats::default();
    let sites = queue::plan(
        rows,
        None,
        Some(3),
        None,
        std::path::Path::new("/tmp/school-sites"),
        &mut stats,
    );
    check!(eq; sites.len(), 3);
    check!(eq; sites[0].school.as_str(), "School 0");
    check!(eq; sites[1].school.as_str(), "School 3");
    check!(eq; sites[2].school.as_str(), "School 6");
    Ok(())
}

#[test]
fn unmapped_states_are_counted_not_crawled() -> TestResult {
    let rows = vec![QueueRow {
        state: "Ontario".to_string(),
        name: "Some High School".to_string(),
        website: "example.ca".to_string(),
    }];
    let mut stats = QueueStats::default();
    let sites = queue::plan(
        rows,
        None,
        None,
        None,
        std::path::Path::new("/tmp/school-sites"),
        &mut stats,
    );
    check!(eq; sites.len(), 0);
    check!(eq; stats.unmapped_state, 1);
    Ok(())
}

#[test]
fn fragment_writer_emits_the_lane_header() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    let signals = Signals {
        emails: vec!["coach@example.k12.us".to_string()],
        coach_hits: vec![hit(
            "Jane Doe",
            Sport::CrossCountry,
            Some("coach@example.k12.us"),
        )],
        ad_hits: Vec::new(),
        pages: vec!["https://example.k12.us/coaches".to_string()],
    };
    let outcome = SiteOutcome {
        signals,
        requests: 1,
        errors: 0,
        note: None,
        pages: vec![PageEvidence {
            url: "https://example.k12.us/coaches".to_string(),
            digest: "digest".to_string(),
            fetched_at: "2026-10-05T04:00:00Z".to_string(),
            status: 200,
        }],
    };
    let planned = site(
        UsJurisdiction::Missouri,
        "Rock Bridge High School",
        "https://example.k12.us",
    );
    let rows = contact_rows(&planned, &outcome);
    let written = artifacts::write_fragments(dir.path(), &rows)?;
    check!(eq; written.len(), 1);
    let mut reader = csv::Reader::from_path(dir.path().join("MO.csv"))?;
    let header = reader.headers()?.clone();
    check!(eq; header.len(), 12);
    check!(eq; header.get(0), Some("school"));
    check!(eq; header.get(11), Some("verified_proof_digest"));
    let record = reader.records().next().ok_or("no fragment row")??;
    check!(eq; record.get(0), Some("Rock Bridge High School"));
    check!(eq; record.get(2), Some("MO"));
    check!(eq; record.get(3), Some("Cross Country"));
    check!(eq; record.get(5), Some("Jane Doe"));
    check!(eq; record.get(10), Some("2026-10-05"));
    Ok(())
}

#[test]
fn site_record_keeps_prototype_keys() -> TestResult {
    let signals = Signals {
        emails: vec!["coach@example.k12.us".to_string()],
        coach_hits: vec![hit("Jane Doe", Sport::CrossCountry, None)],
        ad_hits: Vec::new(),
        pages: vec!["https://example.k12.us/coaches".to_string()],
    };
    let outcome = SiteOutcome {
        signals,
        requests: 1,
        errors: 0,
        note: None,
        pages: Vec::new(),
    };
    let planned = site(
        UsJurisdiction::Missouri,
        "Rock Bridge High School",
        "https://example.k12.us",
    );
    let record = artifacts::site_record(&planned, &outcome);
    let value = serde_json::to_value(&record)?;
    for key in [
        "state",
        "school",
        "website",
        "emails",
        "coach_hits",
        "ad_hits",
        "pages",
    ] {
        check!(value.get(key).is_some(), "missing key {key}");
    }
    check!(value.get("empty").is_none());
    let first = value
        .get("coach_hits")
        .and_then(|hits| hits.get(0))
        .ok_or("no coach hit")?;
    check!(eq; first.get("sport").and_then(|value| value.as_str()), Some("CrossCountry"));
    check!(eq; first.get("role").and_then(|value| value.as_str()), Some("HeadCoach"));
    check!(eq; first.get("gender").and_then(|value| value.as_str()), Some("Girls"));
    Ok(())
}
