use super::write_recruiting_csv;
use crate::export::ExportDataset;
use crate::report::{Derivation, Scope};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, CoachTenure,
    CoachTenureEvidence, Evidence, Gender, GradYear, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

const FORMULA_ATHLETE: &str = "=cmd|' /C calc'!A0";
const FORMULA_SCHOOL: &str = "\t+1+1";
const FORMULA_CITY: &str = "\u{feff}@SUM(1)";
const FORMULA_COACH: &str = "-2+3";
const FORMULA_EMAIL: &str = "=2+2@contacts.test";
const WEBSITE: &str = "https://schools.test/athletics";
const SOURCE_URL: &str = "https://contacts.test/schools";

fn tenure() -> CoachTenureEvidence {
    CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).expect("2026 is a season"),
        },
        source: SourceRef::new("fixture", Some(SOURCE_URL.to_string())),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".to_string(),
        statement: "Synthetic academic-year appointment".to_string(),
    }
}

fn column(header: &csv::StringRecord, name: &str) -> usize {
    header
        .iter()
        .position(|field| field == name)
        .expect("published column")
}

#[test]
fn published_recruiting_csv_literalizes_source_text_without_rewriting_it() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let store = Store::open(directory.path()).expect("store opens");
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        FORMULA_SCHOOL,
        normalize_name(FORMULA_SCHOOL),
    );
    school.city = Some(FORMULA_CITY.to_string());
    school.athletics_website = Some(WEBSITE.to_string());
    store
        .append(Table::Schools, &school)
        .expect("school appends");

    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "csv-safety");
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        FORMULA_ATHLETE,
        GradYear::CO2027,
        Gender::Boys,
        source,
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.evidence = vec![Evidence::parsed(
        SourceRef::new("wiaa_results", None),
        "2026-09-20",
    )];
    store
        .append(Table::Athletes, &athlete)
        .expect("athlete appends");

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
    coach.tenure_evidence = vec![tenure()];
    store.append(Table::Coaches, &coach).expect("coach appends");

    let dataset = ExportDataset::load(&store).expect("frozen export loads");
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027));
    let path = directory.path().join("recruiting.csv");
    let counts = write_recruiting_csv(
        &derivation,
        SchoolYear::new(2026).expect("2026 is a season"),
        &path,
    )
    .expect("recruiting CSV publishes");
    assert_eq!(counts.with_school_coach, 1);
    assert_eq!(counts.with_coach_email, 1);

    let text = std::fs::read_to_string(&path).expect("published CSV reads");
    for formula in [
        FORMULA_ATHLETE,
        FORMULA_SCHOOL,
        FORMULA_CITY,
        FORMULA_COACH,
        FORMULA_EMAIL,
    ] {
        assert!(
            !text.contains(&format!(",{formula}")),
            "{formula:?} must not begin a cell"
        );
        assert!(
            text.contains(&format!("'{formula}")),
            "{formula:?} must keep its literal spelling"
        );
    }

    let mut reader = csv::Reader::from_path(&path).expect("published CSV reads");
    let header = reader.headers().expect("header row").clone();
    let row = reader
        .records()
        .next()
        .expect("one data row")
        .expect("data row decodes");
    for (name, value) in [
        ("name", FORMULA_ATHLETE),
        ("school", FORMULA_SCHOOL),
        ("school_city", FORMULA_CITY),
        ("head_track_coach", FORMULA_COACH),
        ("head_track_coach_email", FORMULA_EMAIL),
    ] {
        assert_eq!(
            row.get(column(&header, name)),
            Some(format!("'{value}").as_str()),
            "{name}"
        );
    }
    assert_eq!(row.get(column(&header, "grad_year")), Some("2027"));
    assert_eq!(row.get(column(&header, "state")), Some("WI"));
    assert_eq!(
        row.get(column(&header, "sports")),
        Some(Sport::OutdoorTrack.stable_key())
    );
    assert_eq!(row.get(column(&header, "athletics_website")), Some(WEBSITE));
    assert_eq!(
        row.get(column(&header, "coach_source_url")),
        Some(SOURCE_URL)
    );
    assert_eq!(
        row.get(column(&header, "evidence_sources")),
        Some("wiaa_results")
    );
}
