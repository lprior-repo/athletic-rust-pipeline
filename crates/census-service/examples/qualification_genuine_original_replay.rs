use census_report::export::{store_lineage, ExportDataset};
use census_report::workbook::publication::{
    current_workbook, recover_publication, verify_published, RecoveryOutcome,
};
use census_store::Store;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

type Error = Box<dyn std::error::Error>;

const STORE: &str = "e42b638b78140cb59542bbce1673d500905b1559021c13c34a24e4ec56716427";
const GENERATION: &str = "0276343dbf363fe810805c4390a2cf14c9442d5cae69904a82e387e4cf313bc8";
const WORKBOOK: &str = "2a3b286b8354fdfa412b86ac128d6f3a03af914a007e4b0e77ed6dbeb026c69c";
const FROZEN: &str = "10c621829e47b9b4a9f31518321439055fada3e30e510700cd890b76183d4983";

fn require_file(path: &Path, digest: &str, length: u64) -> Result<(), Error> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 16384];
    let mut bytes = 0u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes = bytes
            .checked_add(u64::try_from(count)?)
            .ok_or("file length overflow")?;
    }
    if bytes != length || format!("{:x}", hasher.finalize()) != digest {
        return Err(format!("original artifact changed: {}", path.display()).into());
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut args = std::env::args_os().skip(1);
    let first = args.next().ok_or("missing Store root or --verify-only")?;
    if first == "--verify-only" {
        let workbook = PathBuf::from(args.next().ok_or("missing workbook path")?);
        if args.next().is_some() {
            return Err("verify-only requires exactly one workbook path".into());
        }
        let verified = verify_published(&workbook)?;
        if verified.generation_digest != GENERATION || verified.workbook.mapped_athletes != 1 {
            return Err("cold proof control differs from the original generation".into());
        }
        println!(
            "{}",
            serde_json::json!({
                "scope": "Store-free full frozen-proof verification",
                "generation": verified.generation_digest,
                "mapped_athletes": verified.workbook.mapped_athletes,
                "rows": verified.workbook.rows
            })
        );
        return Ok(());
    }
    let store_root = PathBuf::from(first);
    let root = PathBuf::from(args.next().ok_or("missing original publication root")?);
    let generation = args.next().ok_or("missing original generation")?;
    if generation != GENERATION || args.next().is_some() {
        return Err("expected the exact original generation and three arguments".into());
    }
    if !std::fs::metadata(&store_root)?.is_dir() {
        return Err("qualification Store root is not an existing directory".into());
    }
    let directory = root.join("generations").join(GENERATION);
    let workbook = directory.join("workbook.xlsx");
    let frozen = directory.join("frozen-input.json");
    require_file(&workbook, WORKBOOK, 30756)?;
    require_file(&frozen, FROZEN, 262971)?;
    let before_verified = verify_published(&workbook)?;
    if before_verified.generation_digest != GENERATION
        || before_verified.workbook.mapped_athletes != 1
    {
        return Err(
            "cold original publication differs from the qualified athlete generation".into(),
        );
    }
    let dataset = ExportDataset::reopen_frozen_with_digest(&frozen, FROZEN)?;
    if dataset.lineage.store_identity != STORE {
        return Err("original frozen Store identity differs".into());
    }
    let (store, writer) = Store::open_current_qualification_v3(&store_root)?.into_parts();
    drop(writer);
    let before = store_lineage(&store)?;
    if before.store_identity != STORE {
        return Err("foreign Store cannot recover the original request".into());
    }
    let mut outcomes = [String::new(), String::new()];
    for label in &mut outcomes {
        let outcome = recover_publication(&store, &root)?;
        let bundle = match &outcome {
            RecoveryOutcome::HistoricalSuccess { bundle }
            | RecoveryOutcome::DesiredPublished { bundle } => bundle,
            _ => return Err(format!("original request did not recover: {outcome:?}").into()),
        };
        if bundle.as_str() != GENERATION || current_workbook(&root)? != workbook {
            return Err("recovery changed the original generation or workbook identity".into());
        }
        *label = format!("{outcome:?}");
    }
    let after = store_lineage(&store)?;
    if before.input_generation != after.input_generation
        || before.input_digest != after.input_digest
        || before.source_digest != after.source_digest
        || before.snapshot_sequence != after.snapshot_sequence
    {
        return Err("publication recovery changed accepted source inputs".into());
    }
    drop(store);
    let after_verified = verify_published(&workbook)?;
    require_file(&workbook, WORKBOOK, 30756)?;
    require_file(&frozen, FROZEN, 262971)?;
    if after_verified.generation_digest != GENERATION
        || after_verified.workbook.mapped_athletes != 1
    {
        return Err("post-close cold verification changed the original population".into());
    }
    println!(
        "{}",
        serde_json::json!({
            "scope": "genuine one-athlete original publication recovery, not national acceptance",
            "store": STORE,
            "generation": GENERATION,
            "workbook": workbook,
            "workbook_sha256": WORKBOOK,
            "frozen_sha256": FROZEN,
            "outcomes": outcomes,
            "before": before,
            "after": after,
            "cold_verified_before_and_after": true,
            "new_request_or_capture": false
        })
    );
    Ok(())
}
