use census_domain::model::normalize_name;
use census_domain::model::{CanonicalSchool, Evidence, EvidenceMethod, SourceRef};
use census_domain::UsJurisdiction;
use census_store::{Application, Store, StoreError, Table};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BASELINE_COUNT: usize = 4;
const BATCH_COUNT: usize = 32;
const ENOSPC_RETRIES: usize = 128;
const NOTE_BYTES: usize = 64 * 1024;
type ExampleResult<T> = Result<T, Box<dyn Error>>;
type BatchReceipt = (String, u64);
type Failure = (usize, String, bool);
type WriteOutcome = (Vec<BatchReceipt>, Option<Failure>);
fn parse_fs_size(value: &str) -> u64 {
    let (digits, multiplier) = if let Some(value) = value.strip_suffix(['k', 'K']) {
        (value, 1024_u64)
    } else if let Some(value) = value.strip_suffix(['m', 'M']) {
        (value, 1024_u64.saturating_mul(1024))
    } else {
        (value, 1)
    };
    digits
        .parse::<u64>()
        .map_or(0, |size| size.saturating_mul(multiplier))
}
fn check_bounded_fs(root: &Path) -> ExampleResult<()> {
    let root_str = root.to_str().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "tmpfs root is not UTF-8")
    })?;
    let output = Command::new("findmnt")
        .args(["-n", "-o", "OPTIONS", "-T", root_str])
        .output()
        .map_err(|error| std::io::Error::other(format!("findmnt failed: {error}")))?;
    let opts = String::from_utf8_lossy(&output.stdout);
    let opts_str = opts.trim();
    if opts_str.is_empty() {
        return Ok(());
    }
    for part in opts_str.split(',') {
        if let Some(value) = part.strip_prefix("size=") {
            let size = parse_fs_size(value);
            if size > 256_u64.saturating_mul(1024).saturating_mul(1024) {
                return Err(std::io::Error::other(format!(
                    "filesystem cap too large: {} bytes",
                    size
                ))
                .into());
            }
        }
    }
    Ok(())
}
fn high_entropy_note(index: usize, size: usize) -> ExampleResult<String> {
    let mut bytes = Vec::with_capacity(size);
    for i in 0..size {
        let value = index.saturating_mul(257).saturating_add(i);
        let value = value
            .checked_rem(95)
            .ok_or_else(|| std::io::Error::other("entropy modulus was zero"))?
            .saturating_add(32);
        let byte = u8::try_from(value)?;
        bytes.push(byte);
    }
    Ok(String::from_utf8(bytes)?)
}

fn mk_school(index: usize) -> ExampleResult<CanonicalSchool> {
    let name = format!("School_{:04}", index);
    let (mut school, _id) =
        CanonicalSchool::new(UsJurisdiction::Kansas, &name, normalize_name(&name));
    school.evidence.push(Evidence {
        source: SourceRef::id(format!("drill:{}", index)),
        method: EvidenceMethod::Fetched,
        observed_on: "2026-01-01".into(),
        note: Some(high_entropy_note(index, NOTE_BYTES)?),
    });
    Ok(school)
}
fn digest_for(records: &[CanonicalSchool]) -> ExampleResult<String> {
    let mut h = Sha256::new();
    for record in records {
        h.update(serde_json::to_vec(record)?);
    }
    Ok(h.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}
fn is_enospc(err: &StoreError) -> bool {
    let mut current: &dyn Error = err;
    loop {
        if let Some(io) = current.downcast_ref::<std::io::Error>() {
            if io.raw_os_error() == Some(28) {
                return true;
            }
        }
        match current.source() {
            Some(next) => current = next,
            None => return false,
        }
    }
}
fn baseline(store: &Store) -> ExampleResult<()> {
    let schools: Vec<CanonicalSchool> = (0..BASELINE_COUNT)
        .map(mk_school)
        .collect::<ExampleResult<_>>()?;
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &schools)?;
    let op = "baseline";
    let d = digest_for(&schools)?;
    batch.commit_once(op, &d)?;
    let count = store.scan::<CanonicalSchool>(Table::Schools)?.len();
    if count != BASELINE_COUNT {
        return Err(std::io::Error::other(format!(
            "baseline count: got {} expected {}",
            count, BASELINE_COUNT
        ))
        .into());
    }
    println!("PASS: baseline {} schools", BASELINE_COUNT);
    Ok(())
}
fn drain_writes(store: &Store) -> ExampleResult<WriteOutcome> {
    let mut committed: Vec<BatchReceipt> = Vec::new();
    let mut first_failure: Option<Failure> = None;
    for attempt in 0..ENOSPC_RETRIES {
        let start = BASELINE_COUNT.saturating_add(attempt.saturating_mul(BATCH_COUNT));
        let end =
            BASELINE_COUNT.saturating_add(attempt.saturating_add(1).saturating_mul(BATCH_COUNT));
        let schools: Vec<CanonicalSchool> =
            (start..end).map(mk_school).collect::<ExampleResult<_>>()?;
        let mut batch = store.write_batch();
        batch.append_many(Table::Schools, &schools)?;
        let op = format!("batch_{}", attempt);
        let d = digest_for(&schools)?;
        match batch.commit_once(&op, &d) {
            Ok(Application::Written(receipt)) => {
                committed.push((op, receipt.appended));
                let completed = attempt.saturating_add(1);
                if completed % 10 == 0 || attempt == ENOSPC_RETRIES.saturating_sub(1) {
                    println!("committed {} batches so far", completed);
                }
            }
            Ok(Application::Repeated(_)) => {}
            Err(error) => {
                if first_failure.is_none() {
                    let has_enospc = is_enospc(&error);
                    first_failure = Some((attempt, error.to_string(), has_enospc));
                    break;
                }
            }
        }
    }

    if first_failure.is_none() {
        return Err(std::io::Error::other(format!(
            "no failure after {} attempts, committed: {}",
            ENOSPC_RETRIES,
            committed.len()
        ))
        .into());
    }
    Ok((committed, first_failure))
}
fn reopen_rows(root: &Path) -> ExampleResult<Vec<CanonicalSchool>> {
    drop(fs::remove_dir_all(root.join(".enospc_scratch")));
    drop(Store::open(root)?);
    let store = Store::open(root)?;
    Ok(store.scan::<CanonicalSchool>(Table::Schools)?)
}
fn verify_count(all_rows: &[CanonicalSchool], committed: &[BatchReceipt]) -> ExampleResult<()> {
    let baseline = u64::try_from(BASELINE_COUNT)?;
    let total_expected = committed.iter().try_fold(baseline, |total, (_, count)| {
        total
            .checked_add(*count)
            .ok_or_else(|| std::io::Error::other("reopen count exceeded u64"))
    })?;
    let actual = u64::try_from(all_rows.len())?;
    if actual != total_expected {
        return Err(std::io::Error::other(format!(
            "reopen count: got {} expected {}",
            all_rows.len(),
            total_expected
        ))
        .into());
    }
    Ok(())
}
fn verify_baseline_ids(ids: &HashSet<String>) -> ExampleResult<()> {
    for i in 0..BASELINE_COUNT {
        let expected_id = mk_school(i)?.id.as_str().to_string();
        if !ids.contains(&expected_id) {
            return Err(std::io::Error::other(format!("baseline {} missing", i)).into());
        }
    }
    Ok(())
}
fn verify_committed_ids(ids: &HashSet<String>, committed: &[BatchReceipt]) -> ExampleResult<()> {
    let mut expected_start = BASELINE_COUNT;
    for (_, count) in committed {
        let count = usize::try_from(*count)?;
        for j in 0..count {
            let expected_id = mk_school(expected_start.saturating_add(j))?
                .id
                .as_str()
                .to_string();
            if !ids.contains(&expected_id) {
                return Err(
                    std::io::Error::other(format!("committed {} missing", expected_id)).into(),
                );
            }
        }
        expected_start = expected_start.saturating_add(count);
    }
    Ok(())
}

fn verify_failed_ids(ids: &HashSet<String>, failure: &Failure) -> ExampleResult<()> {
    let (fail_idx, _, _) = failure;
    let fail_start = BASELINE_COUNT.saturating_add(fail_idx.saturating_mul(BATCH_COUNT));
    for j in 0..BATCH_COUNT {
        let expected_id = mk_school(fail_start.saturating_add(j))?
            .id
            .as_str()
            .to_string();
        if ids.contains(&expected_id) {
            return Err(std::io::Error::other(format!("failed {} leaked", j)).into());
        }
    }
    Ok(())
}
fn print_verification(committed: &[BatchReceipt], failure: Option<&Failure>) {
    if let Some((fail_idx, err_str, was_enospc)) = failure {
        if *was_enospc {
            println!(
                "PASS: {} baseline, {} batches committed, {} refused (ENOSPC), no duplicates",
                BASELINE_COUNT,
                committed.len(),
                fail_idx
            );
        } else {
            println!(
                "PASS: {} baseline, {} batches committed, {} refused (non-ENOSPC: {}), no duplicates",
                BASELINE_COUNT,
                committed.len(),
                fail_idx,
                err_str
            );
        }
    } else {
        println!(
            "PASS: {} baseline, {} batches committed, no failure",
            BASELINE_COUNT,
            committed.len()
        );
    }
}
fn verify_reopen(
    root: &Path,
    committed: &[BatchReceipt],
    failure: Option<Failure>,
) -> ExampleResult<()> {
    let all_rows = reopen_rows(root)?;
    verify_count(&all_rows, committed)?;
    let ids: HashSet<String> = all_rows
        .iter()
        .map(|school| school.id.as_str().to_string())
        .collect();
    verify_baseline_ids(&ids)?;
    verify_committed_ids(&ids, committed)?;
    if let Some(failure) = &failure {
        verify_failed_ids(&ids, failure)?;
    }
    print_verification(committed, failure.as_ref());
    Ok(())
}
fn main() -> ExampleResult<()> {
    let root: PathBuf = std::env::args()
        .nth(1)
        .ok_or_else(|| std::io::Error::other("usage: enospc <tmpfs-root>"))?
        .into();
    let tmp_name = root.join(".enospc_scratch");
    check_bounded_fs(&root)?;
    fs::create_dir_all(&tmp_name)?;

    let store = Store::open(&root)?;
    baseline(&store)?;
    let (committed, fail) = drain_writes(&store)?;
    drop(store);
    verify_reopen(&root, &committed, fail)
}
