use super::contacts::{contact_rows, csv_sport, real_person};
use super::queue::{self, PlannedSite, QueueStats};
use census_crawl::school_sites::{AdHit, CoachHit, PageEvidence, QueueRow, Signals, SiteOutcome};
use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;

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
    PlannedSite {
        state,
        school: school.to_string(),
        website: website.to_string(),
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
        school_mailboxes: Vec::new(),
        staff_contacts: Vec::new(),
        staff_pages: Vec::new(),
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
    check!(eq; coach.public_professional_email.as_str(), "");
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
fn queue_planning_filters_missing_websites_and_duplicate_subjects() -> TestResult {
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
        (None, None),
        &mut stats,
    )?;
    check!(eq; sites.len(), 3);
    check!(eq; stats.read, 5);
    check!(eq; stats.without_website, 1);
    check!(eq; stats.duplicate, 1);
    check!(eq; sites[0].website.as_str(), "https://example.k12.us");
    check!(eq; sites[1].state, UsJurisdiction::Missouri);
    check!(eq; sites[2].state, UsJurisdiction::California);
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
    let sites = queue::plan(rows, None, (Some(3), None), &mut stats)?;
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
    let sites = queue::plan(rows, None, (None, None), &mut stats)?;
    check!(eq; sites.len(), 0);
    check!(eq; stats.unmapped_state, 1);
    Ok(())
}

#[test]
fn missing_owner_retains_incomplete_site_record() -> TestResult {
    use super::execution::State;

    let temp_dir = tempfile::tempdir()?;
    let out = temp_dir.path().join("out");
    std::fs::create_dir_all(&out)?;

    let planned = site(
        UsJurisdiction::Wisconsin,
        "Fault Probe School",
        "http://127.0.0.1:19505/home",
    );

    let mut state = State::new(out.clone(), QueueStats::default(), 1)?;

    let home = "http://127.0.0.1:19505/home".to_string();
    let mut signals = Signals::default();
    signals.pages.push(home.clone());
    let outcome = SiteOutcome {
        signals,
        requests: 4,
        errors: 3,
        note: Some("http status 429".to_string()),
        pages: vec![PageEvidence {
            url: home.clone(),
            digest: "0".repeat(64),
            fetched_at: "2026-10-11T00:00:00Z".to_string(),
            status: 200,
        }],
    };

    state.missing_site(&planned, &outcome)?;
    let result = state.finish()?;

    check!(eq; result.planned, 1);
    check!(eq; result.crawled, 1);
    check!(eq; result.empty, 0);
    check!(eq; result.failed, 1);
    check!(eq; result.failures.len(), 1);
    check!(eq; result.requests, 4);
    check!(eq; result.errors, 4);

    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("WI__fault-probe-school.json"))?)?;
    check!(eq; record["pages"], serde_json::json!([home]));
    check!(eq; record["page_evidence"].as_array().map(Vec::len), Some(1));
    check!(eq; record["page_evidence"][0]["status"], 200);

    Ok(())
}

#[path = "canonical_tests.rs"]
mod canonical_tests;

#[path = "research_tests.rs"]
mod research_tests;

#[path = "staff_research_tests.rs"]
mod staff_research_tests;

#[path = "contact_slice_tests.rs"]
mod contact_slice_tests;

#[path = "queue_failure_tests.rs"]
mod queue_failure_tests;
