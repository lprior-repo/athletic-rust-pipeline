use super::*;
use sha2::{Digest, Sha256};

pub(super) const CCD_SHA: &str = "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8";
pub(super) const CCD_URL: &str =
    "https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip";
pub(super) const CAPTURE_DATE: &str = "2026-09-28";

pub(super) struct Retained {
    pub(super) generation: PathBuf,
    pub(super) replay: crate::school_address::replay::Replay,
    entries: Vec<SchoolDirectoryEntry>,
}

impl Retained {
    pub(super) fn new(dir: &Path) -> TestResult<Self> {
        let capture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv");
        check!(eq; format!("{:x}", Sha256::digest(std::fs::read(&capture)?)), CCD_SHA);
        let staged = dir.join("nces/ccd_sch_029_2526_head.csv");
        std::fs::create_dir_all(staged.parent().ok_or("no staged parent")?)?;
        std::fs::copy(&capture, &staged)?;
        let manifest = dir.join("replay.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(&json!({
                "schema_revision": 1,
                "month": "2026-10",
                "captures": [{
                    "format": "nces-ccd", "path": "nces/ccd_sch_029_2526_head.csv", "url": CCD_URL,
                    "retrieved_at": CAPTURE_DATE, "sha256": CCD_SHA,
                }],
            }))?,
        )?;
        let generation = dir.join("corpus");
        let replay = crate::school_address::replay::run(&manifest, &generation)?;
        let verified = crate::school_address::verify_current(&generation)?;
        let entries = serde_json::from_slice(verified.artifact("school_directory.json")?)?;
        Ok(Self {
            generation,
            replay,
            entries,
        })
    }

    pub(super) fn school(&self, name: &str) -> TestResult<CanonicalSchool> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.name().is_some_and(|value| value.as_str() == name))
            .ok_or("real CCD fixture lacks named school")?;
        let address = entry.address().ok_or("real CCD entry lacks address")?;
        let state = address.state().ok_or("real CCD address lacks state")?;
        let city = address.city().ok_or("real CCD address lacks city")?;
        let (school, _) =
            CanonicalSchool::new(state, name, normalize_name(name), Some(city.as_str()));
        Ok(school)
    }
}

pub(super) fn ambiguous_index(
    name: &str,
    city: &str,
    state: UsJurisdiction,
) -> TestResult<DirectoryIndex> {
    Ok(DirectoryIndex::build(&[
        entry("390000000002", name, city, state, "1 Main St")?,
        entry("390000000003", name, city, state, "2 Main St")?,
    ]))
}

pub(super) fn advance_without_reviews(store: &Store, label: &str) -> TestResult<u64> {
    let mut stage = store.stage_derived()?;
    for table in Table::ALL {
        if table.generation_partitioned() {
            stage.replace_many::<Value>(table, &[])?;
        }
    }
    let publication = stage.publish(&format!("receipt-fixture:derive:{label}"), label)?;
    check!(publication.application.written());
    Ok(publication.generation)
}
