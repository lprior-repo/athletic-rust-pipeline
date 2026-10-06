use super::write_recruiting_csv;
use crate::export::ExportDataset;
use crate::report::{Derivation, Scope};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachContactClaim,
    CoachContactProgram, CoachRole, CoachTenure, CoachTenureEvidence, Evidence, Gender, GradYear,
    PublishedGraduation, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FORMULA_ATHLETE: &str = "=cmd|' /C calc'!A0";
const FORMULA_SCHOOL: &str = "\t+1+1";
const FORMULA_CITY: &str = "\u{feff}@SUM(1)";
const FORMULA_COACH: &str = "-2+3";
const FORMULA_EMAIL: &str = "=2+2@contacts.test";
const WEBSITE: &str = "https://schools.test/athletics";
const SOURCE_URL: &str = "https://contacts.test/schools";

fn tenure(coach: &CanonicalCoach) -> TestResult<CoachTenureEvidence> {
    Ok(CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new("fixture", Some(SOURCE_URL.to_string())),
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
    })
}

fn column(header: &csv::StringRecord, name: &str) -> TestResult<usize> {
    header
        .iter()
        .position(|field| field == name)
        .ok_or_else(|| format!("missing published column {name}").into())
}

#[test]
fn published_recruiting_csv_literalizes_source_text_without_rewriting_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        FORMULA_SCHOOL,
        normalize_name(FORMULA_SCHOOL),
        Some(FORMULA_CITY),
    );
    school.athletics_website = Some(WEBSITE.to_string());
    store.append(Table::Schools, &school)?;

    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "csv-safety");
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        FORMULA_ATHLETE,
        GradYear::CO2027,
        Gender::Boys,
        source,
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    let claim = PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "wiaa_results",
            Some("https://fixtures.test/recruiting/csv-safety/2027".to_owned()),
        ),
    };
    let mut claim_evidence = Evidence::parsed(claim.source.clone(), "2026-09-20");
    claim_evidence.note = Some("Synthetic public class-of-2027 fixture claim".to_owned());
    athlete.evidence = vec![claim_evidence];
    athlete.published_graduations.push(claim);
    store.append(Table::Athletes, &athlete)?;

    let mut coach = CanonicalCoach::new(
        &school_id,
        FORMULA_COACH,
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some(FORMULA_EMAIL.to_string());
    coach.evidence = vec![Evidence::parsed(
        SourceRef::new("coach_contacts_csv", Some(SOURCE_URL.to_string())),
        "2026-09-20",
    )];
    coach.tenure_evidence = vec![tenure(&coach)?];
    store.append(Table::Coaches, &coach)?;

    let dataset = ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027));
    let path = directory.path().join("recruiting.csv");
    let counts = write_recruiting_csv(
        &derivation,
        SchoolYear::new(2026).ok_or("invalid fixture season")?,
        &path,
    )?;
    check!(eq; counts.with_school_coach, 1);
    check!(eq; counts.with_coach_email, 1);

    let text = std::fs::read_to_string(&path)?;
    for formula in [
        FORMULA_ATHLETE,
        FORMULA_SCHOOL,
        FORMULA_CITY,
        FORMULA_COACH,
        FORMULA_EMAIL,
    ] {
        check!(
            !text.contains(&format!(",{formula}")),
            "{formula:?} must not begin a cell"
        );
        check!(
            text.contains(&format!("'{formula}")),
            "{formula:?} must keep its literal spelling"
        );
    }

    let mut reader = csv::Reader::from_path(&path)?;
    let header = reader.headers()?.clone();
    let row = reader.records().next().ok_or("missing CSV data row")??;
    for (name, value) in [
        ("name", FORMULA_ATHLETE),
        ("school", FORMULA_SCHOOL),
        ("school_city", FORMULA_CITY),
        ("head_track_coach", FORMULA_COACH),
        ("head_track_coach_email", FORMULA_EMAIL),
    ] {
        check!(eq; row.get(column(&header, name)?),
        Some(format!("'{value}").as_str()),
        "{name}");
    }
    check!(eq; row.get(column(&header, "grad_year")?), Some("2027"));
    check!(eq; row.get(column(&header, "state")?), Some("WI"));
    check!(eq; row.get(column(&header, "sports")?),
    Some(Sport::OutdoorTrack.stable_key()));
    check!(eq; row.get(column(&header, "athletics_website")?),
    Some(WEBSITE));
    check!(eq; row.get(column(&header, "coach_source_url")?),
    Some(SOURCE_URL));
    check!(eq; row.get(column(&header, "evidence_sources")?),
    Some("wiaa_results"));
    Ok(())
}
