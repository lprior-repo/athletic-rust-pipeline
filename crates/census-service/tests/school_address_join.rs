#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::school_directory::SchoolDirectoryEntry;
use census_domain::UsJurisdiction;
use census_service::school_address::{self, join_generation, Mode, Overrides, SchoolAddressArgs};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type ClaimRow = (String, String, String, String);

const KINGSTON_ID: &str = "800000038718";
const AVON_ID: &str = "800000054526";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/state_ed")
        .join(name)
}

fn avon_profile(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join("profile_avon.html");
    let page = std::fs::read_to_string(fixture("profile_kingston.html"))?;
    std::fs::write(
        &path,
        page.replace(KINGSTON_ID, AVON_ID)
            .replace("A A KINGSTON MIDDLE SCHOOL", "AVON PRIMARY SCHOOL")
            .replace(
                "29+LEROY+ST%2C+POTSDAM%2C+NY%2C+13676",
                "1+AVON+WAY%2C+AVON%2C+NY%2C+14414",
            )
            .replace("https://www.potsdamcsd.org", "https://www.avoncsd.org")
            .replace("512902060004", "512902060005"),
    )?;
    Ok(path)
}

fn args(out: &Path, indexes: &[PathBuf], profiles: &[PathBuf]) -> SchoolAddressArgs {
    SchoolAddressArgs {
        ccd: None,
        pss: None,
        state_ed_index: indexes.to_vec(),
        state_ed_profile: profiles.to_vec(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        association_directory: Vec::new(),
        out: out.into(),
        baseline: None,
        ledger: None,
        now: Some("2026-10".to_string()),
        geocode: false,
        validate_postal: false,
    }
}

fn publish(out: &Path, indexes: &[PathBuf], profiles: &[PathBuf]) -> TestResult {
    school_address::run(&args(out, indexes, profiles))?;
    Ok(())
}

fn corpus(out: &Path) -> Result<Vec<SchoolDirectoryEntry>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(out.join("current/school_directory.json"))?;
    Ok(serde_json::from_str(&text)?)
}

fn seed(
    root: &Path,
    entries: &[SchoolDirectoryEntry],
    only: &[&str],
) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::open(root)?;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        let Some(name) = entry.name() else { continue };
        if !only.is_empty() && !only.contains(&name.as_str()) {
            continue;
        }
        if !seen.insert(name.as_str().to_string()) {
            continue;
        }
        let city = entry.address().and_then(|address| address.city());
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::NewYork,
            name.as_str(),
            normalize_name(name.as_str()),
            city.map(|value| value.as_str()),
        );
        store.append(Table::Schools, &school)?;
    }
    Ok(store)
}

fn claims(store: &Store) -> TestResult<Vec<ClaimRow>> {
    let mut rows = Vec::new();
    for school in store.scan::<CanonicalSchool>(Table::Schools)? {
        for claim in &school.postal_addresses {
            rows.push((
                school.name.clone(),
                claim.capture_sha256().to_string(),
                claim
                    .evidence()
                    .source
                    .url
                    .as_deref()
                    .map_or(String::new(), str::to_string),
                claim
                    .evidence()
                    .note
                    .as_deref()
                    .map_or(String::new(), str::to_string),
            ));
        }
    }
    rows.sort();
    Ok(rows)
}

fn file_sha(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}

fn overrides(captures: &[(&Path, String)]) -> Overrides {
    Overrides {
        urls: captures
            .iter()
            .map(|(path, url)| (format!("state-ed@{}", path.display()), url.clone()))
            .collect(),
        dates: BTreeMap::from([("state-ed".to_string(), "2026-09-01".to_string())]),
    }
}

fn without_generation_digest(note: &str) -> &str {
    match note.split(" generation ").next() {
        Some(head) => head,
        None => note,
    }
}

#[test]
fn each_profile_capture_credits_its_own_bytes_whatever_the_argument_order() -> TestResult {
    let parent = TempDir::new()?;
    let avon = avon_profile(parent.path())?;
    let kingston = fixture("profile_kingston.html");
    let kingston_url = format!("https://data.nysed.gov/profile.php?instid={KINGSTON_ID}");
    let avon_url = format!("https://data.nysed.gov/profile.php?instid={AVON_ID}");
    let mut runs = Vec::new();
    let mut digests = Vec::new();
    for reversed in [false, true] {
        let generation = parent
            .path()
            .join(if reversed { "reversed" } else { "declared" });
        let order = if reversed {
            vec![avon.clone(), kingston.clone()]
        } else {
            vec![kingston.clone(), avon.clone()]
        };
        publish(&generation, &[], &order)?;
        let store = seed(&generation.join("store"), &corpus(&generation)?, &[])?;
        let report = join_generation(
            &store,
            &generation,
            Some(&generation.join("out")),
            overrides(&[
                (kingston.as_path(), kingston_url.clone()),
                (avon.as_path(), avon_url.clone()),
            ]),
            Mode::Apply,
        )?;
        check!(eq; report.counters.linked, 2);
        check!(eq; report.counters.refused, 0);
        check!(eq; report.counters.evidence_missing, 0);
        let rows = claims(&store)?;
        check!(eq; rows.len(), 2);
        let kingston_row = rows
            .iter()
            .find(|row| row.0 == "A A KINGSTON MIDDLE SCHOOL")
            .ok_or("no Kingston claim")?;
        check!(eq; kingston_row.1, file_sha(&kingston)?);
        check!(eq; kingston_row.2, kingston_url);
        check!(kingston_row.3.contains(&kingston.display().to_string()));
        let avon_row = rows
            .iter()
            .find(|row| row.0 == "AVON PRIMARY SCHOOL")
            .ok_or("no Avon claim")?;
        check!(eq; avon_row.1, file_sha(&avon)?);
        check!(eq; avon_row.2, avon_url);
        check!(avon_row.3.contains(&avon.display().to_string()));
        check!(!avon_row.3.contains(&kingston.display().to_string()));
        check!(!kingston_row.3.contains(&avon.display().to_string()));
        digests.push(std::fs::read_link(generation.join("current"))?);
        runs.push(
            rows.into_iter()
                .map(|row| {
                    (
                        row.0,
                        row.1,
                        row.2,
                        without_generation_digest(&row.3).to_string(),
                    )
                })
                .collect::<Vec<_>>(),
        );
    }
    check!(
        digests[0] != digests[1],
        "the two orders published distinct generations"
    );
    check!(eq; runs[0].len(), 2);
    check!(eq; runs[0], runs[1]);
    Ok(())
}

#[test]
fn two_captures_carrying_one_school_refuse_the_link() -> TestResult {
    let parent = TempDir::new()?;
    let generation = parent.path().join("generation");
    publish(
        &generation,
        &[fixture("index_letter_a.html")],
        &[fixture("profile_kingston.html")],
    )?;
    let store = seed(
        &generation.join("store"),
        &corpus(&generation)?,
        &["A A KINGSTON MIDDLE SCHOOL"],
    )?;
    let index = fixture("index_letter_a.html");
    let kingston = fixture("profile_kingston.html");
    let report = join_generation(
        &store,
        &generation,
        Some(&generation.join("out")),
        overrides(&[
            (
                index.as_path(),
                format!("https://data.nysed.gov/index-letter-a?instid={KINGSTON_ID}"),
            ),
            (
                kingston.as_path(),
                format!("https://data.nysed.gov/profile.php?instid={KINGSTON_ID}"),
            ),
        ]),
        Mode::Apply,
    )?;
    check!(eq; report.counters.linked, 0);
    check!(eq; report.counters.refused, 1);
    let outcomes = std::fs::read_to_string(&report.outcomes)?;
    check!(outcomes.contains("several captures of state-ed"));
    check!(outcomes.contains("index_letter_a.html"));
    check!(outcomes.contains("profile_kingston.html"));
    check!(eq; claims(&store)?.len(), 0);
    Ok(())
}
