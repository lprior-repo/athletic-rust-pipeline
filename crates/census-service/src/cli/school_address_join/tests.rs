use super::*;
use census_domain::model::{
    normalize_name, serialized_digest, CanonicalSchool, CensusRun, GradYear, RunManifest,
    SchoolYear, SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_service::school_address::{
    accept_nces_address_run, NcesSelectedArtifactV1, NcesSelectedInputV1,
};
use census_store::Table;
use clap::Parser;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    _parent: tempfile::TempDir,
    root: PathBuf,
    generation: PathBuf,
}

fn fixture() -> TestResult<Fixture> {
    let parent = tempfile::tempdir()?;
    let root = parent.path().join("store");
    let inputs = parent.path().join("inputs");
    std::fs::create_dir(&inputs)?;
    std::fs::write(
        inputs.join("ccd.csv"),
        include_bytes!("../../../tests/fixtures/nces_run/ccd_synthetic.csv"),
    )?;
    std::fs::write(
        inputs.join("pss.csv"),
        include_bytes!("../../../tests/fixtures/nces_run/pss_synthetic.csv"),
    )?;
    Store::create_current_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let scope = vec![
        UsJurisdiction::Colorado,
        UsJurisdiction::Illinois,
        UsJurisdiction::Iowa,
        UsJurisdiction::Michigan,
        UsJurisdiction::Minnesota,
    ];
    let manifest = RunManifest {
        store_identity: store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("school year")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: scope.clone(),
    };
    store.bind_run(&manifest)?;
    accept_nces_address_run(
        &store,
        &NcesSelectedInputV1 {
            source_root: inputs,
            run_manifest_digest: serialized_digest(&manifest)?,
            scope,
            ccd: artifact(
                "ccd.csv",
                "57c07542819bb066c4ec68e20ab2eb53d32eeccd84408e457418fa0ecd37f94d",
            ),
            pss: artifact(
                "pss.csv",
                "777b31b02fb7f47652049f8f6ad81a8b1f8f875aaa4ff948f173fea4cfe1adcf",
            ),
        },
    )?;
    let name = "Synthetic Michigan Public School";
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        name,
        normalize_name(name),
        Some("Testville"),
    );
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::school_directory("nces-ccd", UsJurisdiction::Michigan),
        "260000000001",
    ));
    store.append(Table::Schools, &school)?;
    let generation = store.out_dir().join("nces-address");
    drop((store, writer));
    Ok(Fixture {
        _parent: parent,
        root,
        generation,
    })
}

fn artifact(file: &str, sha256: &str) -> NcesSelectedArtifactV1 {
    NcesSelectedArtifactV1 {
        path: PathBuf::from(file),
        sha256: sha256.to_string(),
        source_url: format!("https://fixtures.invalid/nces/{file}"),
        captured_on: "2026-10-23".to_string(),
    }
}

fn execute(root: &Path, generation: &Path, apply: bool) -> TestResult {
    let mut args = vec![
        "census-service",
        "--store",
        root.to_str().ok_or("root")?,
        "school-address-join",
        "--generation",
        generation.to_str().ok_or("generation")?,
    ];
    if apply {
        args.push("--apply");
    }
    let cli = Cli::try_parse_from(args)?;
    match &cli.command {
        crate::cli::Command::SchoolAddressJoin(args) => Ok(run(&cli, args)?),
        other => Err(format!("unexpected command {other:?}").into()),
    }
}

#[test]
fn selected_address_join_uses_verified_receipt_for_dry_run_apply_and_replay() -> TestResult {
    let given = fixture()?;
    std::fs::write(
        given.generation.join("replay-evidence.json"),
        b"untrusted loose evidence",
    )?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&given.root)?.into_parts();
    let before = store.snapshot().sequence();
    drop((store, writer));
    execute(&given.root, &given.generation, false)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&given.root)?.into_parts();
    check!(eq; store.snapshot().sequence(), before);
    drop((store, writer));
    execute(&given.root, &given.generation, true)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&given.root)?.into_parts();
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let claims: Vec<_> = schools
        .iter()
        .flat_map(|school| &school.postal_addresses)
        .collect();
    check!(eq; claims.len(), 1);
    for claim in claims {
        check!(eq; claim.evidence().source.url.as_deref(), Some("https://fixtures.invalid/nces/ccd.csv"));
        check!(eq; claim.evidence().observed_on.as_str(), "2026-10-23");
        check!(eq; claim.capture_sha256(), "57c07542819bb066c4ec68e20ab2eb53d32eeccd84408e457418fa0ecd37f94d");
    }
    let applied = store.snapshot().sequence();
    drop((store, writer));
    execute(&given.root, &given.generation, true)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&given.root)?.into_parts();
    check!(eq; store.snapshot().sequence(), applied);
    drop((store, writer));
    Ok(())
}

#[test]
fn selected_address_join_refuses_missing_store_without_creating_it() -> TestResult {
    let parent = tempfile::tempdir()?;
    let root = parent.path().join("absent");
    check!(execute(&root, parent.path(), true).is_err());
    check!(!root.exists());
    Ok(())
}

fn assert_unchanged(given: &Fixture, sequence: u64) -> TestResult {
    let (store, writer) = Store::open_public_identity_qualification_v3(&given.root)?.into_parts();
    check!(eq; store.snapshot().sequence(), sequence);
    check!(store
        .scan::<CanonicalSchool>(Table::Schools)?
        .iter()
        .all(|school| school.postal_addresses.is_empty()));
    drop((store, writer));
    Ok(())
}

#[test]
fn selected_address_join_refuses_tampered_retained_capture_before_writing_claims() -> TestResult {
    let given = fixture()?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&given.root)?.into_parts();
    let receipt = store.nces_address_run_v1()?.ok_or("receipt")?;
    let sequence = store.snapshot().sequence();
    drop((store, writer));
    let capture = given
        .generation
        .join("captures")
        .join(receipt.input_manifest_digest())
        .join(receipt.ccd().sha256());
    std::fs::write(capture, b"altered capture")?;
    check!(execute(&given.root, &given.generation, true).is_err());
    assert_unchanged(&given, sequence)
}

#[test]
fn selected_address_join_refuses_another_valid_generation_before_writing_claims() -> TestResult {
    let given = fixture()?;
    let other = given._parent.path().join("other-generation");
    let ccd = given._parent.path().join("inputs/ccd.csv");
    let pss = given._parent.path().join("inputs/pss.csv");
    let cli = Cli::try_parse_from([
        "census-service",
        "school-address",
        "--ccd",
        ccd.to_str().ok_or("CCD")?,
        "--pss",
        pss.to_str().ok_or("PSS")?,
        "--out",
        other.to_str().ok_or("other")?,
    ])?;
    match &cli.command {
        crate::cli::Command::SchoolAddress(args) => census_service::school_address::run(args)?,
        other => return Err(format!("unexpected command {other:?}").into()),
    }
    census_service::school_address::verify_current(&other)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&given.root)?.into_parts();
    let sequence = store.snapshot().sequence();
    drop((store, writer));
    check!(execute(&given.root, &other, true).is_err());
    assert_unchanged(&given, sequence)
}
