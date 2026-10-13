#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::{net::FetchOutcome, uhsaa};
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachTenure, SchoolYear};
use census_domain::school_directory::SchoolDirectoryEntry;
use census_service::school_address::{self, join_generation, Mode, Overrides, SchoolAddressArgs};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const URL: &str = "https://uhsaa.org/school-directory/?Reg=6&id=Alta&schoolID=1";
const REPLAYED_CAPTURE_TIME: &str = "2026-10-04T00:00:00Z";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/uhsaa")
        .join(name)
}

fn args(out: &Path, name: &str) -> SchoolAddressArgs {
    SchoolAddressArgs {
        ccd: None,
        pss: None,
        state_ed_index: Vec::new(),
        state_ed_profile: Vec::new(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        association_directory: vec![format!("uhsaa={}", fixture(name).display())],
        out: out.to_path_buf(),
        baseline: None,
        ledger: None,
        now: Some("2026-10".to_string()),
        geocode: false,
        validate_postal: false,
    }
}

#[test]
fn school_address_uhsaa_retained_readback_preserves_postal_and_withholds_unseasoned_tenure(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let corpus = root.path().join("corpus");
    school_address::run(&args(&corpus, "directory.html"))?;
    let generation = school_address::verify_current(&corpus)?;
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(generation.artifact("school_directory.json")?)?;
    check!(eq; entries.len(), 162);
    check!(entries.iter().all(|entry| entry.address().is_none()));
    let indexed =
        uhsaa::parse_directory_links(&std::fs::read_to_string(fixture("directory.html"))?)
            .into_iter()
            .find(|link| link.name == "Alta Hawks")
            .ok_or("Alta directory owner")?;
    let body = std::fs::read(fixture("profile-alta.html"))?;
    let digest = format!("{:x}", Sha256::digest(&body));
    let capture = FetchOutcome {
        url: URL.to_string(),
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: digest.clone(),
        bytes: body.len(),
        fetched_at: REPLAYED_CAPTURE_TIME.to_string(),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body,
    };
    let extracted = uhsaa::map_capture(&indexed, &capture)?;
    let store = Store::open(root.path().join("store"))?;
    store.append(Table::Schools, &extracted.school)?;
    for coach in &extracted.coaches {
        store.append(Table::Coaches, coach)?;
    }
    let report = join_generation(
        &store,
        &corpus,
        None,
        Overrides {
            acquisitions: vec![census_service::school_address::Acquisition {
                entry: 0,
                provider: "association:uhsaa".to_string(),
                path: String::new(),
                url: "https://uhsaa.org/school-directory-new/".to_string(),
                retrieved_at: Some(REPLAYED_CAPTURE_TIME.to_string()),
            }],
        },
        Mode::Apply,
    )?;
    check!(eq; report.counters.scanned, 1);
    check!(eq; report.counters.postal_filled, 0);
    check!(eq; report.counters.postal_gaps, 0);
    let mut schools = Vec::new();
    store.for_each_merged(Table::Schools, |school: CanonicalSchool| {
        schools.push(school);
        Ok(())
    })?;
    let school = schools.first().ok_or("joined school readback")?;
    check!(eq; school.postal_addresses, extracted.school.postal_addresses);
    let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
    check!(eq; coaches.len(), 4);
    for coach in coaches {
        check!(eq; coach.tenure_state(SchoolYear::new(2026).ok_or("year")?)?, CoachTenure::Unknown);
    }
    let claim = school
        .postal_addresses
        .first()
        .ok_or("typed postal readback")?;
    check!(eq; claim.owner().id.as_str(), "1");
    check!(eq; claim.capture_sha256(), digest.as_str());
    check!(eq; claim.evidence().observed_on.as_str(), REPLAYED_CAPTURE_TIME);
    println!("UHSAA retained-capture readback: entries={} postal_claims={} postal_filled={} tenure=Unknown capture_sha256={digest} replay_metadata_time={REPLAYED_CAPTURE_TIME}",
        entries.len(), school.postal_addresses.len(), report.counters.postal_filled);
    Ok(())
}

#[test]
fn school_address_uhsaa_ownerless_profile_refuses_corpus_publication() -> TestResult {
    let root = tempfile::tempdir()?;
    let error = school_address::run(&args(root.path(), "profile-alta.html"))
        .err()
        .ok_or("ownerless profile was admitted")?;
    check!(format!("{error:#}").contains("no published directory school owners"));
    check!(eq; root.path().join("current").try_exists()?, false);
    Ok(())
}
