use super::{fixture, TestResult};
use census_domain::model::{serialized_digest, ProfileSelectionV1, ProvisionalInclusionV1};
use census_report::export::{ExportDataset, ExportJobRequest};
use census_report::workbook::{self, publication};
use census_service::restate_services::{workbook_request_key, ExportGeneration, WorkbookRequest};
use census_store::{publish::Digest, Store};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn selection(fixture: &fixture::Fixture) -> TestResult<ProfileSelectionV1> {
    let mut input = fixture.profile.input().clone();
    input.include_provisional = ProvisionalInclusionV1::Include;
    let path = fixture.select_profile(input, "derivation-selection.json")?;
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn request(original: &Path, selection: &ProfileSelectionV1) -> WorkbookRequest {
    WorkbookRequest {
        grad_year: None,
        limit: None,
        scope: Some("all_sources".into()),
        out: Some(original.to_string_lossy().into_owned()),
        school_year: Some(2026),
        jurisdictions: vec![
            census_domain::UsJurisdiction::Alabama,
            census_domain::UsJurisdiction::Wisconsin,
        ],
        export_job: ExportJobRequest::SelectedPublicationV1 {
            selection: selection.clone(),
        },
    }
}

fn publish_original(
    fixture: &fixture::Fixture,
    original: &Path,
    selection: &ProfileSelectionV1,
) -> TestResult<PathBuf> {
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    let native = request(original, selection);
    let job = format!(
        "workbook:{}",
        workbook_request_key(&native, &ExportGeneration::parse("retained")?)?
    );
    let dataset = ExportDataset::for_job_request(&store, &job, &native.export_job)?;
    let requested = workbook::Options {
        address_corpus: None,
        grad_year: None,
        out: Some(original.to_path_buf()),
        limit: None,
        scope: census_report::report::Scope::AllSources,
        jurisdictions: fixture.profile.input().jurisdictions.clone(),
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
    };
    let options = workbook::options_for_selected_request(&store, selection, &requested)?;
    let censuses = workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    Ok(workbook::build_from(&dataset, &store, &options, &censuses)?)
}

fn derive(
    fixture: &fixture::Fixture,
    selection: &ProfileSelectionV1,
    original: &Path,
    derived: &Path,
    generation: &str,
) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(&fixture.root)
        .args([
            "workbook",
            "--school-year",
            "2026",
            "--resume-frozen",
            "--generation",
            generation,
            "--states",
            "AL,WI",
        ])
        .arg("--selected-store-identity")
        .arg(selection.store_identity().as_str())
        .arg("--selected-profile-id")
        .arg(selection.profile_id().as_str())
        .arg("--out")
        .arg(original)
        .arg("--derive-out")
        .arg(derived)
        .output()?)
}

fn inventory(root: &Path) -> TestResult<BTreeMap<PathBuf, String>> {
    let mut directories = vec![root.to_path_buf()];
    let mut files = BTreeMap::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let relative = entry.path().strip_prefix(root)?.to_path_buf();
            if kind.is_dir() {
                directories.push(entry.path());
            } else if kind.is_symlink() {
                files.insert(
                    relative,
                    serialized_digest(&std::fs::read_link(entry.path())?)?,
                );
            } else {
                files.insert(relative, serialized_digest(&std::fs::read(entry.path())?)?);
            }
        }
    }
    Ok(files)
}

fn active(fixture: &fixture::Fixture, root: &Path) -> TestResult<String> {
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    let namespace = Digest::parse(&serialized_digest(&std::fs::canonicalize(root)?)?)?;
    Ok(serde_json::to_string(
        &store
            .publication_active(&namespace)?
            .ok_or("publication receipt")?,
    )?)
}

fn manifest(path: &Path) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(
        path.parent().ok_or("generation")?.join("manifest.json"),
    )?)?)
}

#[tokio::test]
async fn retained_derivation_preserves_original_input_files_receipt_and_replays_cold() -> TestResult
{
    let fixture = fixture::Fixture::create().await?;
    let selection = selection(&fixture)?;
    let original = fixture.directory.path().join("original");
    let derived = fixture.directory.path().join("derived");
    let predecessor = publish_original(&fixture, &original, &selection)?;
    let before = inventory(&original)?;
    let receipt = active(&fixture, &original)?;
    let inputs = inventory(&fixture.root.join("out/export-inputs"))?;
    let evidence = fixture.evidence_state()?;
    let result = derive(&fixture, &selection, &original, &derived, "retained")?;
    check!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let path = std::fs::canonicalize(derived.join("current/workbook.xlsx"))?;
    let verified = publication::verify_published(&path)?;
    let old = manifest(&predecessor)?;
    let current = manifest(&path)?;
    check!(eq; current["schema_revision"], 12);
    check!(eq; current["lineage"], old["lineage"]);
    check!(eq; current["selected_profile"], old["selected_profile"]);
    let original_input = old
        .pointer("/artifacts/frozen-input.json")
        .ok_or("original frozen input commitment")?;
    check!(eq; current.pointer("/artifacts/frozen-input.json"), Some(original_input));
    let proofs = inventory(
        &predecessor
            .parent()
            .ok_or("original generation")?
            .join("frozen-input.json.proofs"),
    )?;
    check!(!proofs.is_empty());
    check!(eq; inventory(&path.parent().ok_or("derived generation")?.join("frozen-input.json.proofs"))?, proofs);
    check!(eq; inventory(&original)?, before);
    check!(eq; active(&fixture, &original)?, receipt);
    check!(active(&fixture, &derived)? != receipt);
    check!(eq; inventory(&fixture.root.join("out/export-inputs"))?, inputs);
    check!(eq; fixture.evidence_state()?, evidence);
    let published = inventory(path.parent().ok_or("derived generation")?)?;
    let derived_receipt = active(&fixture, &derived)?;
    let repeated = derive(&fixture, &selection, &original, &derived, "retained")?;
    check!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    check!(eq; std::fs::canonicalize(derived.join("current/workbook.xlsx"))?, path);
    check!(eq; inventory(path.parent().ok_or("derived generation")?)?, published);
    check!(eq; active(&fixture, &derived)?, derived_receipt);
    check!(eq; inventory(&fixture.root.join("out/export-inputs"))?, inputs);
    check!(eq; inventory(&original)?, before);
    std::fs::remove_dir_all(&fixture.root)?;
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    Ok(())
}

#[tokio::test]
async fn retained_derivation_refuses_missing_original_job_before_creating_output() -> TestResult {
    let fixture = fixture::Fixture::create().await?;
    let selection = selection(&fixture)?;
    let original = fixture.directory.path().join("original");
    let derived = fixture.directory.path().join("derived");
    publish_original(&fixture, &original, &selection)?;
    let before = inventory(&original)?;
    let inputs = inventory(&fixture.root.join("out/export-inputs"))?;
    let result = derive(&fixture, &selection, &original, &derived, "absent-job")?;
    check!(!result.status.success());
    check!(String::from_utf8_lossy(&result.stderr)
        .contains("resuming the existing native workbook input"));
    check!(!derived.exists());
    check!(eq; inventory(&original)?, before);
    let after = inventory(&fixture.root.join("out/export-inputs"))?;
    for (name, bytes) in &inputs {
        check!(eq; after.get(name), Some(bytes));
    }
    for (name, _) in after.iter().filter(|(name, _)| !inputs.contains_key(*name)) {
        check!(eq; name.extension(), Some(std::ffi::OsStr::new("lock")));
        check!(eq; std::fs::metadata(fixture.root.join("out/export-inputs").join(name))?.len(), 0);
    }
    Ok(())
}

#[tokio::test]
async fn retained_derivation_refuses_changed_source_without_recapture_or_output() -> TestResult {
    let fixture = fixture::Fixture::create().await?;
    let selection = selection(&fixture)?;
    let original = fixture.directory.path().join("original");
    let derived = fixture.directory.path().join("derived");
    publish_original(&fixture, &original, &selection)?;
    let before = inventory(&original)?;
    let inputs = inventory(&fixture.root.join("out/export-inputs"))?;
    {
        let (store, _writer) =
            Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
        let (school, _) = census_domain::model::CanonicalSchool::new(
            census_domain::UsJurisdiction::Alabama,
            "Changed Source School",
            "changed source school",
            None,
        );
        store.append(census_store::Table::Schools, &school)?;
    }
    let evidence = fixture.evidence_state()?;
    let result = derive(&fixture, &selection, &original, &derived, "retained")?;
    check!(!result.status.success());
    check!(String::from_utf8_lossy(&result.stderr).contains("stale or foreign export input"));
    check!(!derived.exists());
    check!(eq; inventory(&original)?, before);
    check!(eq; inventory(&fixture.root.join("out/export-inputs"))?, inputs);
    check!(eq; fixture.evidence_state()?, evidence);
    Ok(())
}

#[tokio::test]
async fn retained_derivation_does_not_decode_controlled_unsupported_predecessor_bytes() -> TestResult
{
    let fixture = fixture::Fixture::create().await?;
    let selection = selection(&fixture)?;
    let original = fixture.directory.path().join("original");
    let derived = fixture.directory.path().join("derived");
    let predecessor = publish_original(&fixture, &original, &selection)?;
    let manifest_path = predecessor
        .parent()
        .ok_or("generation")?
        .join("manifest.json");
    std::fs::write(
        manifest_path,
        br#"{"schema_revision":9,"controlled_opaque_predecessor":true}"#,
    )?;
    let before = inventory(&original)?;
    let receipt = active(&fixture, &original)?;
    let result = derive(&fixture, &selection, &original, &derived, "retained")?;
    check!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    publication::verify_published(&std::fs::canonicalize(
        derived.join("current/workbook.xlsx"),
    )?)?;
    check!(eq; inventory(&original)?, before);
    check!(eq; active(&fixture, &original)?, receipt);
    Ok(())
}

#[tokio::test]
async fn retained_derivation_refuses_damaged_retained_proof_without_repairing_it() -> TestResult {
    for remove in [false, true] {
        let fixture = fixture::Fixture::create().await?;
        let selection = selection(&fixture)?;
        let original = fixture.directory.path().join("original");
        let derived = fixture.directory.path().join("derived");
        publish_original(&fixture, &original, &selection)?;
        let before = inventory(&original)?;
        let input_root = fixture.root.join("out/export-inputs");
        let inputs = inventory(&input_root)?;
        let proof = inputs
            .keys()
            .find(|path| {
                path.components()
                    .any(|component| component.as_os_str() == "input.json.proofs")
            })
            .ok_or("retained proof object")?;
        if remove {
            std::fs::remove_file(input_root.join(proof))?;
        } else {
            std::fs::write(input_root.join(proof), b"controlled damaged retained proof")?;
        }
        let damaged = inventory(&input_root)?;
        let result = derive(&fixture, &selection, &original, &derived, "retained")?;
        check!(!result.status.success());
        check!(String::from_utf8_lossy(&result.stderr)
            .contains("resuming the existing native workbook input"));
        check!(!derived.exists());
        check!(eq; inventory(&original)?, before);
        check!(eq; inventory(&input_root)?, damaged);
    }
    Ok(())
}
