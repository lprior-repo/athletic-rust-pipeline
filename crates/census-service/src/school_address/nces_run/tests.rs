use super::*;
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};
use census_domain::school_directory::SchoolDirectoryEntry;
use std::collections::BTreeSet;

mod retention_bounds;
mod selected_scope;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CCD: &[u8] = include_bytes!("../../../tests/fixtures/nces_run/ccd_synthetic.csv");
const PSS: &[u8] = include_bytes!("../../../tests/fixtures/nces_run/pss_synthetic.csv");

const CCD_SHA256: &str = "57c07542819bb066c4ec68e20ab2eb53d32eeccd84408e457418fa0ecd37f94d";
const PSS_SHA256: &str = "777b31b02fb7f47652049f8f6ad81a8b1f8f875aaa4ff948f173fea4cfe1adcf";

struct Fixture {
    _root: tempfile::TempDir,
    source_root: PathBuf,
    store_path: PathBuf,
    store: Store,
    run_digest: String,
    selection: NcesSelectedInputV1,
}

fn fixture() -> TestResult<Fixture> {
    let root = tempfile::tempdir()?;
    let source_root = root.path().join("inputs");
    std::fs::create_dir(&source_root)?;
    std::fs::write(source_root.join("ccd.csv"), CCD)?;
    std::fs::write(source_root.join("pss.csv"), PSS)?;

    let store_path = root.path().join("store");
    let store = Store::open(&store_path)?;
    let manifest = RunManifest {
        store_identity: store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("school year")?, 1)
            .ok_or("run revision")?,
        cohort: GradYear::CO2027,
        jurisdictions: FIVE_STATE_SCOPE.to_vec(),
    };
    store.bind_run(&manifest)?;
    let run_digest = serialized_digest(&manifest)?;
    let selection = NcesSelectedInputV1 {
        source_root: source_root.clone(),
        run_manifest_digest: run_digest.clone(),
        scope: FIVE_STATE_SCOPE.to_vec(),
        ccd: NcesSelectedArtifactV1 {
            path: PathBuf::from("ccd.csv"),
            sha256: CCD_SHA256.to_string(),
            source_url: "https://fixtures.invalid/nces/ccd.csv".to_string(),
            captured_on: "2026-10-20".to_string(),
        },
        pss: NcesSelectedArtifactV1 {
            path: PathBuf::from("pss.csv"),
            sha256: PSS_SHA256.to_string(),
            source_url: "https://fixtures.invalid/nces/pss.csv".to_string(),
            captured_on: "2026-10-20".to_string(),
        },
    };
    Ok(Fixture {
        _root: root,
        source_root,
        store_path,
        store,
        run_digest,
        selection,
    })
}

fn assert_no_writes(fixture: &Fixture, sequence: u64) -> TestResult {
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    check!(fixture
        .store
        .scan::<census_domain::model::CanonicalSchool>(census_store::Table::Schools)?
        .is_empty());
    check!(eq; fixture.store.nces_address_run_v1()?, None);
    check!(!fixture.store.out_dir().join("nces-address").exists());
    Ok(())
}

#[test]
fn nces_address_run_accepts_exact_hash_pinned_five_state_inputs_and_reopens_frozen_output(
) -> TestResult {
    let fixture = fixture()?;
    check!(eq; verified_input::sha256_hex(CCD), CCD_SHA256);
    check!(eq; verified_input::sha256_hex(PSS), PSS_SHA256);
    let receipt = accept(&fixture.store, &fixture.selection)?;
    check!(eq; receipt.run_manifest_digest(), fixture.run_digest);
    check!(eq; receipt.scope(), FIVE_STATE_SCOPE);
    check!(eq; receipt.ccd().sha256(), CCD_SHA256);
    check!(eq; receipt.pss().sha256(), PSS_SHA256);
    check!(eq; fixture.store.nces_address_run_v1()?, Some(receipt.clone()));
    let committed_sequence = fixture.store.snapshot().sequence();
    check!(eq; accept(&fixture.store, &fixture.selection)?, receipt);
    check!(eq; fixture.store.snapshot().sequence(), committed_sequence);

    std::fs::remove_file(fixture.source_root.join("ccd.csv"))?;
    std::fs::remove_file(fixture.source_root.join("pss.csv"))?;
    drop(fixture.store);
    let reopened_store = Store::open(&fixture.store_path)?;
    let (reopened_receipt, generation) = reopen(&reopened_store, &fixture.run_digest)?;
    check!(eq; reopened_receipt, receipt);
    let report: super::super::Report =
        serde_json::from_slice(generation.artifact("pipeline_report.json")?)?;
    check!(eq; report.lanes.len(), 2);
    check!(eq; report.manifest_digest, reopened_receipt.address_generation_digest());
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(generation.artifact("school_directory.json")?)?;
    check!(eq; entries.len(), 10);
    check!(entries.iter().all(|entry| entry.address().is_some()));
    let states: BTreeSet<UsJurisdiction> = entries
        .iter()
        .map(|entry| entry.jurisdiction().ok_or("entry has no state"))
        .collect::<std::result::Result<_, _>>()?;
    check!(eq; states, FIVE_STATE_SCOPE.into_iter().collect());
    Ok(())
}

#[test]
fn nces_address_run_missing_file_refuses_before_store_or_output_writes() -> TestResult {
    let mut fixture = fixture()?;
    fixture.selection.ccd.path = PathBuf::from("missing.csv");
    let sequence = fixture.store.snapshot().sequence();
    check!(accept(&fixture.store, &fixture.selection).is_err());
    assert_no_writes(&fixture, sequence)
}

#[test]
fn nces_address_run_wrong_run_digest_refuses_before_store_or_output_writes() -> TestResult {
    let mut fixture = fixture()?;
    fixture.selection.run_manifest_digest = "0".repeat(64);
    let sequence = fixture.store.snapshot().sequence();
    check!(accept(&fixture.store, &fixture.selection).is_err());
    assert_no_writes(&fixture, sequence)
}

#[test]
fn nces_address_run_hash_mismatch_refuses_before_store_or_output_writes() -> TestResult {
    let mut fixture = fixture()?;
    fixture.selection.pss.sha256 = "0".repeat(64);
    let sequence = fixture.store.snapshot().sequence();
    check!(accept(&fixture.store, &fixture.selection).is_err());
    assert_no_writes(&fixture, sequence)
}

#[test]
fn nces_address_run_changed_provenance_refuses_without_rewriting_existing_generation() -> TestResult
{
    let mut fixture = fixture()?;
    let receipt = accept(&fixture.store, &fixture.selection)?;
    let out = fixture.store.out_dir().join("nces-address");
    let current = std::fs::read_link(out.join("current"))?;
    let sequence = fixture.store.snapshot().sequence();
    fixture.selection.ccd.source_url = "https://fixtures.invalid/nces/changed-ccd.csv".to_string();
    check!(accept(&fixture.store, &fixture.selection).is_err());
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    check!(eq; fixture.store.nces_address_run_v1()?, Some(receipt));
    check!(eq; std::fs::read_link(out.join("current"))?, current);
    Ok(())
}
#[test]
fn accepted_nces_run_joins_into_canonical_schools_and_survives_frozen_dataset_reopen() -> TestResult
{
    use census_domain::model::{normalize_name, CanonicalSchool, SourceIdentity, SourceNamespace};
    use census_store::Table;

    let fixture = fixture()?;
    let schools = [
        (
            UsJurisdiction::Michigan,
            "Synthetic Michigan Public School",
            "nces-ccd",
            "260000000001",
            "1 Synthetic Way",
            "48001",
        ),
        (
            UsJurisdiction::Minnesota,
            "Synthetic Minnesota Public School",
            "nces-ccd",
            "270000000001",
            "2 Synthetic Way",
            "55001",
        ),
        (
            UsJurisdiction::Iowa,
            "Synthetic Iowa Public School",
            "nces-ccd",
            "190000000001",
            "3 Synthetic Way",
            "50001",
        ),
        (
            UsJurisdiction::Illinois,
            "Synthetic Illinois Public School",
            "nces-ccd",
            "170000000001",
            "4 Synthetic Way",
            "60001",
        ),
        (
            UsJurisdiction::Colorado,
            "Synthetic Colorado Public School",
            "nces-ccd",
            "080000000001",
            "5 Synthetic Way",
            "80001",
        ),
        (
            UsJurisdiction::Michigan,
            "Synthetic Michigan Private School",
            "nces-pss",
            "SYNMI001",
            "11 Synthetic Way",
            "48002",
        ),
        (
            UsJurisdiction::Minnesota,
            "Synthetic Minnesota Private School",
            "nces-pss",
            "SYNMN001",
            "12 Synthetic Way",
            "55002",
        ),
        (
            UsJurisdiction::Iowa,
            "Synthetic Iowa Private School",
            "nces-pss",
            "SYNIA001",
            "13 Synthetic Way",
            "50002",
        ),
        (
            UsJurisdiction::Illinois,
            "Synthetic Illinois Private School",
            "nces-pss",
            "SYNIL001",
            "14 Synthetic Way",
            "60002",
        ),
        (
            UsJurisdiction::Colorado,
            "Synthetic Colorado Private School",
            "nces-pss",
            "SYNCO001",
            "15 Synthetic Way",
            "80002",
        ),
    ];
    for &(jurisdiction, name, provider, source_id, _, _) in &schools {
        let (school, _) =
            CanonicalSchool::new(jurisdiction, name, normalize_name(name), Some("Testville"));
        let mut school = school;
        school.source_identities.push(SourceIdentity::new(
            SourceNamespace::school_directory(provider, jurisdiction),
            source_id,
        ));
        fixture.store.append(Table::Schools, &school)?;
    }
    let receipt = accept(&fixture.store, &fixture.selection)?;
    let generation = fixture.store.out_dir().join("nces-address");
    let overrides = super::super::Overrides::from_acquisitions(vec![
        super::super::Acquisition {
            entry: 0,
            provider: "nces-ccd".to_string(),
            path: String::new(),
            url: receipt.ccd().source_url().to_string(),
            retrieved_at: Some(receipt.ccd().captured_on().to_string()),
        },
        super::super::Acquisition {
            entry: 1,
            provider: "nces-pss".to_string(),
            path: String::new(),
            url: receipt.pss().source_url().to_string(),
            retrieved_at: Some(receipt.pss().captured_on().to_string()),
        },
    ])?;
    let joined = super::super::join_generation_pinned(
        &fixture.store,
        &generation,
        overrides,
        receipt.address_generation_digest(),
    )?;
    check!(eq; (joined.counters.already_linked, joined.counters.backfilled,
        joined.counters.postal_filled, joined.counters.postal_remaining), (10, 10, 10, 0));

    let dataset = census_report::export::ExportDataset::load(&fixture.store)?;
    let claims: usize = dataset
        .schools
        .values()
        .map(|school| school.postal_addresses.len())
        .sum();
    check!(eq; claims, 10);
    let claim_facts = |dataset: &census_report::export::ExportDataset| -> TestResult<_> {
        dataset
            .schools
            .values()
            .flat_map(|school| school.postal_addresses.iter())
            .map(|claim| {
                Ok((
                    claim.owner().namespace.to_string(),
                    claim.owner().id.clone(),
                    claim
                        .address()
                        .line1()
                        .ok_or("address street")?
                        .as_str()
                        .to_string(),
                    claim
                        .address()
                        .city()
                        .ok_or("address city")?
                        .as_str()
                        .to_string(),
                    claim.address().state().ok_or("address state")?,
                    claim
                        .address()
                        .zip()
                        .ok_or("address ZIP")?
                        .code()
                        .to_string(),
                    claim.capture_sha256().to_string(),
                ))
            })
            .collect::<TestResult<BTreeSet<_>>>()
    };
    let expected: BTreeSet<_> = schools
        .iter()
        .map(|(jurisdiction, _, provider, source_id, street, zip)| {
            (
                SourceNamespace::school_directory(provider, *jurisdiction).to_string(),
                (*source_id).to_string(),
                (*street).to_string(),
                "Testville".to_string(),
                *jurisdiction,
                (*zip).to_string(),
                if *provider == "nces-ccd" {
                    CCD_SHA256.to_string()
                } else {
                    PSS_SHA256.to_string()
                },
            )
        })
        .collect();
    check!(eq; claim_facts(&dataset)?, expected);
    let frozen = fixture._root.path().join("joined-dataset.json");
    dataset.save_frozen(&frozen)?;
    let reopened = census_report::export::ExportDataset::reopen_frozen(&frozen)?;
    let reopened_claims: usize = reopened
        .schools
        .values()
        .map(|school| school.postal_addresses.len())
        .sum();
    check!(eq; reopened_claims, 10);
    check!(eq; claim_facts(&reopened)?, expected);
    Ok(())
}
