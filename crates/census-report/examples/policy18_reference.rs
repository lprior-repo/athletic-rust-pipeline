use census_domain::model::{serialized_bytes, RecruitingCatalogV1};
use census_report::export::ExportDataset;
use census_report::report::EventAdmission;
use census_report::workbook::publication::verify_published;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;

type Outcome<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    original_source_pin: String,
    input_generation: String,
    catalog_identity_bytes: String,
    event_audit_bytes: String,
    artifacts: BTreeMap<String, String>,
}

fn main() -> Outcome {
    let args: Vec<_> = std::env::args_os().take(6).collect();
    match args.as_slice() {
        [_, command, workbook, reference, pin] if command == "capture" => {
            capture(Path::new(workbook), Path::new(reference), pin.to_str().ok_or("source pin UTF-8")?)
        }
        [_, command, workbook, reference, pin] if command == "verify" => {
            verify(Path::new(workbook), Path::new(reference), pin.to_str().ok_or("source pin UTF-8")?)
        }
        _ => Err("usage: policy18_reference capture|verify WORKBOOK REFERENCE_JSON ORIGINAL_SOURCE_PIN_SHA256".into()),
    }
}

fn capture(workbook: &Path, destination: &Path, pin: &str) -> Outcome {
    check_pin(pin)?;
    let directory = workbook
        .parent()
        .ok_or("workbook directory")?
        .canonicalize()?;
    let target_parent = destination
        .parent()
        .ok_or("reference parent")?
        .canonicalize()?;
    if target_parent.starts_with(&directory) {
        return Err("reference must be outside the retained generation".into());
    }
    let before = inventory(&directory)?;
    let dataset = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
    require_legacy(&dataset)?;
    let mut identity_bytes = Vec::new();
    RecruitingCatalogV1::new().write_policy(&mut identity_bytes)?;
    let identity: serde_json::Value = serde_json::from_slice(&identity_bytes)?;
    if identity.get("policy_revision").is_some() {
        return Err("capture requires the pinned original policy18 implementation".into());
    }
    let identity_bytes = String::from_utf8(identity_bytes)?;
    check_catalog(&dataset, &identity_bytes)?;
    require_fixture(&dataset)?;
    let reference = Reference {
        original_source_pin: pin.into(),
        input_generation: dataset.lineage.input_generation.clone(),
        catalog_identity_bytes: identity_bytes,
        event_audit_bytes: event_audit(&dataset)?,
        artifacts: before,
    };
    drop(dataset);
    verify_published(workbook)?;
    if inventory(&directory)? != reference.artifacts {
        return Err("legacy verification changed reference artifacts".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    file.write_all(&serialized_bytes(&reference)?)?;
    file.sync_all()?;
    Ok(())
}

fn verify(workbook: &Path, reference_path: &Path, pin: &str) -> Outcome {
    check_pin(pin)?;
    let reference: Reference = serde_json::from_slice(&read_reference(reference_path)?)?;
    if reference.original_source_pin != pin {
        return Err(
            "reference source pin does not match the requested original implementation".into(),
        );
    }
    let directory = workbook.parent().ok_or("workbook directory")?;
    if inventory(directory)? != reference.artifacts {
        return Err("reference artifacts differ before verification".into());
    }
    let dataset = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
    require_legacy(&dataset)?;
    if dataset.lineage.input_generation != reference.input_generation {
        return Err("reference input generation differs".into());
    }
    check_catalog(&dataset, &reference.catalog_identity_bytes)?;
    require_fixture(&dataset)?;
    if event_audit(&dataset)? != reference.event_audit_bytes {
        return Err("historical event admission differs from the original audit".into());
    }
    drop(dataset);
    verify_published(workbook)?;
    if inventory(directory)? != reference.artifacts {
        return Err("historical verification changed retained bytes".into());
    }
    Ok(())
}

fn require_legacy(dataset: &ExportDataset) -> Outcome {
    if dataset.lineage.policy_revision != 18 {
        return Err("reference must contain a genuine policy18 input".into());
    }
    Ok(())
}

fn check_catalog(dataset: &ExportDataset, identity_bytes: &str) -> Outcome {
    let audit = serde_json::to_value(EventAdmission::of(dataset)?.audit())?;
    let digest = format!("{:x}", Sha256::digest(identity_bytes.as_bytes()));
    if audit
        .get("catalog_digest")
        .and_then(serde_json::Value::as_str)
        != Some(digest.as_str())
    {
        return Err("legacy catalog identity bytes do not match the retained calculation".into());
    }
    Ok(())
}

fn event_audit(dataset: &ExportDataset) -> Outcome<String> {
    let admission = EventAdmission::of(dataset)?;
    Ok(String::from_utf8(serialized_bytes(&admission.audit())?)?)
}

fn require_fixture(dataset: &ExportDataset) -> Outcome {
    let expected = [
        "300m",
        "500m",
        "600m",
        "2 miles",
        "3200m",
        "3000m",
        "300m Hurdles",
        "Cross Country 2 miles",
        "1000m (300m banked)",
        "400m banked track",
        "300m (200m banked)",
    ];
    let events = dataset
        .events
        .iter()
        .filter(|event| {
            event
                .source_labels
                .iter()
                .any(|label| label.source.id == "policy18_fixture")
        })
        .collect::<Vec<_>>();
    if events.len() != expected.len() {
        return Err("reference must retain exactly eleven policy18 fixture events".into());
    }
    expected.iter().try_for_each(|label| -> Outcome {
        let count = events
            .iter()
            .filter(|event| {
                event
                    .source_labels
                    .iter()
                    .any(|source| source.source.id == "policy18_fixture" && source.label == *label)
            })
            .count();
        if count != 1 {
            return Err(format!("reference fixture label missing or duplicated: {label}").into());
        }
        Ok(())
    })?;
    events.iter().try_for_each(|event| -> Outcome {
        let count = dataset
            .performances
            .iter()
            .filter(|performance| {
                performance.event == event.id
                    && performance
                        .source_athlete
                        .as_ref()
                        .is_some_and(|source| source.id == "policy18-fixture-runner")
            })
            .count();
        if count != 1 {
            return Err(
                "reference fixture event must retain exactly one runner performance".into(),
            );
        }
        Ok(())
    })
}

fn check_pin(pin: &str) -> Outcome {
    if pin.len() != 64 || !pin.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("original source pin must be a SHA256 digest".into());
    }
    Ok(())
}

fn read_reference(path: &Path) -> Outcome<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > 1_048_576 {
        return Err("reference exceeds one MiB".into());
    }
    let mut bytes = Vec::new();
    file.take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("reference grew beyond one MiB".into());
    }
    Ok(bytes)
}

fn inventory(directory: &Path) -> Outcome<BTreeMap<String, String>> {
    let mut inventory = BTreeMap::new();
    for entry in std::fs::read_dir(directory)?.take(129) {
        if inventory.len() == 128 {
            return Err("reference exceeds 128 artifacts".into());
        }
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err("reference contains a non-file artifact".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "artifact filename UTF-8")?;
        inventory.insert(name, hash(&entry.path())?);
    }
    Ok(inventory)
}

fn hash(path: &Path) -> Outcome<String> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    for _ in 0..3_276_801_u64 {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(format!("{:x}", digest.finalize()));
        }
        digest.update(buffer.get(..count).ok_or("invalid read length")?);
    }
    Err("reference artifact exceeds 200 GiB hashing bound".into())
}
