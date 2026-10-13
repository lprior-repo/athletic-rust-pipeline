use calamine::Reader;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CoachContactClaim, CoachContactProgram, CoachRole, CoachTenure,
    CoachTenureEvidence, CompetitionLevel, EventSpecification, Evidence, ExactSeconds, Gender,
    GradYear, Mark, PerformanceIdentity, PerformanceResult, PublishedGraduation, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use crate::export::ExportDataset;
use crate::workbook::Options;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod bests;
mod postal;

fn seed(store: &Store) -> TestResult {
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Ada High",
        "ada high",
        Some("Madison"),
    );
    school.athletics_website = Some("https://ada.test/athletics".to_string());
    school.evidence = vec![Evidence::parsed(
        SourceRef::new("wiaa_results", Some("https://wiaa.test/ada".to_string())),
        "2026-06-01",
    )];
    store.append(Table::Schools, &school)?;

    let mut coach = CanonicalCoach::new(
        &school_id,
        "Coach Carter",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("carter@ada.test".to_string());
    coach.evidence = vec![Evidence::parsed(
        SourceRef::new(
            "coach_contacts_csv",
            Some("https://contacts.test/ada".to_string()),
        ),
        "2026-09-20",
    )];
    coach.tenure_evidence = vec![CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new("fixture", Some("https://contacts.test/ada".to_string())),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".to_string(),
        statement: "Synthetic academic-year appointment".to_string(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: coach.school.clone(),
            role: coach.role,
            program: CoachContactProgram::Team {
                sport: Sport::OutdoorTrack,
                gender: coach.gender,
            },
            mailbox: coach.professional_email.clone(),
        }),
    }];
    store.append(Table::Coaches, &coach)?;

    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "ada-runner"),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "synthetic_published_roster",
            Some("https://example.invalid/roster/ada-runner".to_string()),
        ),
    });
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.public_profile_urls = vec!["https://www.athletic.net/athlete/1".to_string()];
    athlete.evidence = vec![Evidence::parsed(
        SourceRef::new("wiaa_results", Some("https://wiaa.test/ada".to_string())),
        "2026-05-02",
    )];
    store.append(Table::Athletes, &athlete)?;
    Ok(())
}

fn publish(
    directory: &tempfile::TempDir,
) -> TestResult<(std::path::PathBuf, ExportDataset, Options)> {
    let store = Store::open(directory.path().join("store"))?;
    seed(&store)?;
    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    let published = crate::workbook::build(&store, &options)?;
    let generation = published
        .parent()
        .ok_or("missing generation directory")?
        .to_path_buf();
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    Ok((generation, dataset, options))
}

fn rewrite_cell(path: &std::path::Path, column: &str, value: &str) -> TestResult {
    let mut reader = csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    let index = header
        .iter()
        .position(|name| name == column)
        .ok_or_else(|| format!("missing published column {column}"))?;
    let mut rows: Vec<Vec<String>> = reader
        .records()
        .map(|record| record.map(|record| record.iter().map(str::to_owned).collect()))
        .collect::<Result<_, _>>()?;
    let row = rows.first_mut().ok_or("missing CSV data row to forge")?;
    let cell = row.get_mut(index).ok_or("missing CSV cell to forge")?;
    *cell = value.to_string();
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record(&header)?;
    for row in &rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}

#[test]
fn valid_generated_sidecars_pass_semantic_readback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    super::verify(&generation, &dataset, &options)?;
    Ok(())
}

#[test]
fn a_forged_census_sidecar_fails_semantic_readback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    let path = generation.join("census-core.json");
    let bytes = std::fs::read(&path)?;
    let mut report: serde_json::Value = serde_json::from_slice(&bytes)?;
    report["totals"]["athletes"] = serde_json::Value::from(9_999_u64);
    std::fs::write(&path, serde_json::to_vec(&report)?)?;
    let error = match super::verify(&generation, &dataset, &options) {
        Err(error) => error,
        Ok(()) => return Err("forged census accepted".into()),
    };
    check!(error.to_string().contains("census-core.json"), "{error}");
    Ok(())
}

#[test]
fn a_forged_recruiting_sidecar_fails_semantic_readback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    rewrite_cell(
        &generation.join("recruiting.csv"),
        "name",
        "Mallory Impostor",
    )?;
    let error = match super::verify(&generation, &dataset, &options) {
        Err(error) => error,
        Ok(()) => return Err("forged recruiting row accepted".into()),
    };
    check!(error.to_string().contains("recruiting.csv"), "{error}");
    Ok(())
}

#[test]
fn extra_and_missing_bundle_sidecars_are_rejected() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (generation, dataset, options) = publish(&directory)?;
    let jsonl = generation.join("best-results-co2027.jsonl");
    let mut text = std::fs::read_to_string(&jsonl)?;
    text.push_str("{\"forged\":true}\n");
    std::fs::write(&jsonl, text)?;
    check!(super::verify(&generation, &dataset, &options).is_err());

    std::fs::remove_file(&jsonl)?;
    check!(super::verify(&generation, &dataset, &options).is_err());

    std::fs::remove_file(generation.join("recruiting.csv"))?;
    check!(super::verify(&generation, &dataset, &options).is_err());
    Ok(())
}

#[test]
fn contact_mailboxes_and_research_cannot_borrow_frozen_source_claims() -> TestResult {
    for (name, column, forged) in [
        ("school-contacts.csv", "Mailbox", "borrowed@other.edu"),
        ("school-contacts.csv", "Capture SHA256", "other-capture"),
        ("school-contacts.csv", "Contact State", "current_claim"),
        (
            "school-contacts.csv",
            "Retained Claims",
            "[{\"borrowed\":true}]",
        ),
        (
            "contact-research.csv",
            "Research Outcome",
            "completed_empty",
        ),
        (
            "contact-research.csv",
            "Retained Research",
            "[{\"borrowed\":true}]",
        ),
    ] {
        let directory = tempfile::tempdir()?;
        let (generation, dataset, options) = publish(&directory)?;
        rewrite_cell(&generation.join(name), column, forged)?;
        check!(matches!(
            super::verify(&generation, &dataset, &options),
            Err(crate::report::ReportError::Invariant { .. })
        ));
    }
    Ok(())
}
fn seed_complete(store: &Store) -> TestResult {
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Complete High",
        "complete high",
        Some("Milwaukee"),
    );
    school.athletics_website = Some("https://complete.test/athletics".to_string());
    school.evidence = vec![Evidence::parsed(
        SourceRef::new(
            "wiaa_results",
            Some("https://wiaa.test/complete".to_string()),
        ),
        "2026-06-01",
    )];

    let owner = SourceIdentity::new(SourceNamespace::association_school("ncaa"), "complete-high");
    school.source_identities.push(owner.clone());
    let address = census_domain::school_directory::PostalAddress::of(
        Some(census_domain::school_directory::StreetLine::parse(
            "123 Athletic Drive",
        )?),
        None,
        Some(census_domain::school_directory::CityName::parse(
            "Milwaukee",
        )?),
        Some(UsJurisdiction::Wisconsin),
        Some(census_domain::school_directory::ZipCode::parse("53201")?),
    )
    .ok_or("postal fixture has no address components")?;
    school.add_postal_address(
        census_domain::model::SchoolPostalAddress::new(
            address,
            owner,
            census_domain::school_directory::SourceLabel::AthleticAssociation {
                state: UsJurisdiction::Wisconsin,
            },
            Evidence::parsed(
                SourceRef::new(
                    "ncaa",
                    Some("https://ncaa.test/schools/complete-high".to_string()),
                ),
                "2026-06-01",
            ),
            "b".repeat(64),
        )
        .map_err(|e| format!("postal address: {}", e))?,
    )?;
    store.append(Table::Schools, &school)?;

    let mut coach = CanonicalCoach::new(
        &school_id,
        "Coach Professional",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("coach@complete.test".to_string());
    let claim = CoachContactClaim {
        coach: coach.id.clone(),
        school: coach.school.clone(),
        role: coach.role,
        program: CoachContactProgram::Team {
            sport: Sport::OutdoorTrack,
            gender: coach.gender,
        },
        mailbox: coach.professional_email.clone(),
    };
    coach.tenure_evidence = vec![CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new(
            "coach_contacts_csv",
            Some("https://contacts.test/complete".to_string()),
        ),
        source_sha256: "c".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".to_string(),
        statement: "Published head coach for outdoor track".to_string(),
        claim: Some(claim),
    }];
    store.append(Table::Coaches, &coach)?;

    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "complete-runner");
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Complete Runner",
        GradYear::CO2027,
        Gender::Boys,
        source.clone(),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "synthetic_published_roster",
            Some("https://example.invalid/roster/complete-runner".to_string()),
        ),
    });
    athlete.evidence = vec![Evidence::parsed(
        SourceRef::new(
            "wiaa_results",
            Some("https://wiaa.test/complete".to_string()),
        ),
        "2026-05-02",
    )];
    store.append(Table::Athletes, &athlete)?;

    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Complete Invite",
        "2026-05-02",
        CompetitionLevel::Invitational,
    );
    meet.sports = vec![Sport::OutdoorTrack];
    store.append(Table::Meets, &meet)?;

    let event = crate::test_events::owned(CanonicalEvent::new(
        census_domain::model::EventIdentity {
            meet: &meet.id,
            kind: census_domain::model::EventKind::Track100m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    let team = census_domain::model::TeamId::mint("tm", &[school_id.as_str(), "boys", "track"]);
    store.append(Table::Events, &event)?;

    let mut perf = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            meet: &meet.id,
            event: &event.id,
            date: "2026-05-02",
            source_key: "complete-runner-perf-1",
        },
        PerformanceResult {
            mark: Mark::TimeSeconds(ExactSeconds::parse("10.50")?),
            adjustment: None,
            team: &team,
            wind_mps: Some(1.0),
            place: Some(1),
        },
    )?;
    perf.timing = Some(TimingMethod::Fat);
    store.append(Table::Performances, &perf)?;

    Ok(())
}

#[test]
fn workbook_publishes_one_accepted_athlete_with_complete_joined_details() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed_complete(&store)?;

    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    crate::workbook::build(&store, &options)?;

    let path =
        crate::workbook::publication::current_workbook(&store.out_dir().join("publication"))?;

    let mut book: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
    let athletes = book.worksheet_range("Athletes")?;

    let headers: Vec<String> = athletes
        .rows()
        .next()
        .ok_or("header row missing")?
        .iter()
        .map(|cell| match cell {
            calamine::Data::String(s) => s.to_string(),
            _ => String::new(),
        })
        .collect();
    check!(headers.contains(&"Name".to_string()));
    check!(headers.contains(&"School".to_string()));
    check!(headers.contains(&"School City".to_string()));
    check!(headers.contains(&"School Address".to_string()));
    check!(headers.contains(&"Head TF Coach".to_string()));
    check!(headers.contains(&"Head TF Coach Email".to_string()));
    check!(headers.contains(&"Preferred Recruiting Contact".to_string()));
    check!(headers.contains(&"Preferred Contact Email".to_string()));
    check!(headers.contains(&"Preferred Contact Coach ID".to_string()));
    check!(headers.contains(&"Preferred Contact Source URL".to_string()));
    check!(headers.contains(&"Preferred Contact Capture SHA256".to_string()));
    check!(headers.contains(&"Preferred Contact Acquired At".to_string()));
    check!(headers.contains(&"Identity Status".to_string()));
    check!(headers.contains(&"Coverage State".to_string()));

    let name_col = headers
        .iter()
        .position(|h| h == "Name")
        .ok_or("Name column missing")?;
    let athlete_row = athletes
        .rows()
        .position(|row| {
            matches!(row.get(name_col), Some(calamine::Data::String(s)) if s == "Complete Runner")
        })
        .ok_or("Complete Runner not found in workbook")?;

    let name_cell = athletes
        .get_value((athlete_row as u32, name_col as u32))
        .map(|v| v.to_string())
        .ok_or("name cell missing")?;
    check!(
        name_cell == "Complete Runner",
        "athlete name must be rendered: {}",
        name_cell
    );

    let school_col = headers
        .iter()
        .position(|h| h == "School")
        .ok_or("School column missing")?;
    let school_cell = athletes
        .get_value((1, school_col as u32))
        .map(|v| v.to_string())
        .ok_or("school cell missing")?;
    check!(
        school_cell.contains("Complete High"),
        "school name must be rendered: {}",
        school_cell
    );

    let address_col = headers
        .iter()
        .position(|h| h == "School Address")
        .ok_or("School Address column missing")?;
    let address_cell = athletes
        .get_value((1, address_col as u32))
        .map(|v| v.to_string())
        .ok_or("address cell missing")?;
    check!(
        !address_cell.is_empty(),
        "school postal address must be joined for accepted athlete, was blank"
    );
    check!(
        address_cell.contains("Milwaukee"),
        "postal address must contain city: {}",
        address_cell
    );

    let coach_col = headers
        .iter()
        .position(|h| h == "Head TF Coach")
        .ok_or("Head TF Coach column missing")?;
    let coach_cell = athletes
        .get_value((1, coach_col as u32))
        .map(|v| v.to_string())
        .ok_or("coach cell missing")?;
    check!(
        !coach_cell.is_empty(),
        "head coach name must be rendered for accepted athlete with published coach"
    );

    let email_col = headers
        .iter()
        .position(|h| h == "Head TF Coach Email")
        .ok_or("Head TF Coach Email column missing")?;
    let email_cell = athletes
        .get_value((1, email_col as u32))
        .map(|v| v.to_string())
        .ok_or("email cell missing")?;
    check!(
        !email_cell.is_empty(),
        "head coach email must be rendered for accepted athlete with published coach email"
    );
    check!(
        email_cell.contains("coach@complete.test"),
        "coach email must be joined: {}",
        email_cell
    );

    let preferred_col = headers
        .iter()
        .position(|h| h == "Preferred Recruiting Contact")
        .ok_or("Preferred Recruiting Contact column missing")?;
    let preferred_cell = athletes
        .get_value((1, preferred_col as u32))
        .map(|v| v.to_string())
        .ok_or("preferred contact cell missing")?;
    check!(
        !preferred_cell.is_empty(),
        "preferred recruiting contact must be resolved for accepted athlete with coach evidence"
    );

    let source_col = headers
        .iter()
        .position(|h| h == "Preferred Contact Source URL")
        .ok_or("Preferred Contact Source URL column missing")?;
    let source_cell = athletes
        .get_value((1, source_col as u32))
        .map(|v| v.to_string())
        .ok_or("source URL cell missing")?;
    check!(
        !source_cell.is_empty(),
        "coach source URL must be rendered for accepted athlete"
    );

    let sha_col = headers
        .iter()
        .position(|h| h == "Preferred Contact Capture SHA256")
        .ok_or("Preferred Contact Capture SHA256 column missing")?;
    let sha_cell = athletes
        .get_value((1, sha_col as u32))
        .map(|v| v.to_string())
        .ok_or("capture SHA cell missing")?;
    check!(
        !sha_cell.is_empty(),
        "coach capture SHA256 must be rendered for accepted athlete"
    );

    let coverage_col = headers
        .iter()
        .position(|h| h == "Coverage State")
        .ok_or("Coverage State column missing")?;
    let coverage_cell = athletes
        .get_value((1, coverage_col as u32))
        .map(|v| v.to_string())
        .ok_or("coverage cell missing")?;
    check!(
        coverage_cell == "pr",
        "athlete with performance and PR must have coverage state 'pr': {}",
        coverage_cell
    );

    let status_col = headers
        .iter()
        .position(|h| h == "Identity Status")
        .ok_or("Identity Status column missing")?;
    let status_cell = athletes
        .get_value((1, status_col as u32))
        .map(|v| v.to_string())
        .ok_or("status cell missing")?;
    check!(
        !status_cell.is_empty(),
        "identity status must be rendered for accepted athlete, was blank"
    );

    Ok(())
}
