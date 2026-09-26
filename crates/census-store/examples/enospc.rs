use census_domain::model::{CanonicalSchool, Evidence, EvidenceMethod, SourceRef};
use census_domain::model::normalize_name;
use census_domain::UsJurisdiction;
use census_store::{Application, Store, StoreError, Table};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BASELINE_COUNT: usize = 4;
const BATCH_COUNT: usize = 32;
const MAX_ATTEMPTS: usize = 128;
const NOTE_BYTES: usize = 64 * 1024;

fn check_bounded_fs(root: &Path) {
    let output = Command::new("findmnt")
        .args(["-n", "-o", "OPTIONS", "-T", root.to_str().unwrap()])
        .output()
        .expect("findmnt failed");
    let opts = String::from_utf8_lossy(&output.stdout);
    let opts_str = opts.trim();
    if opts_str.is_empty() {
        return;
    }
    for part in opts_str.split(',') {
        if part.starts_with("size=") {
            let val = &part[5..];
            let size = if val.ends_with('k') || val.ends_with('K') {
                val[..val.len()-1].parse::<u64>().unwrap_or(0) * 1024
            } else if val.ends_with('m') || val.ends_with('M') {
                val[..val.len()-1].parse::<u64>().unwrap_or(0) * 1024 * 1024
            } else {
                val.parse::<u64>().unwrap_or(0)
            };
            assert!(size <= 256 * 1024 * 1024,
                "filesystem cap too large: {} bytes", size);
        }
    }
}

fn high_entropy_note(index: usize, size: usize) -> String {
    let mut bytes = Vec::with_capacity(size);
    for i in 0..size {
        let b = ((index * 257 + i) % 95 + 32) as u8;
        bytes.push(b);
    }
    String::from_utf8(bytes).expect("utf8")
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn mk_school(index: usize) -> CanonicalSchool {
    let name = format!("School_{:04}", index);
    let (mut school, _id) = CanonicalSchool::new(UsJurisdiction::Kansas, &name, normalize_name(&name));
    school.evidence.push(Evidence {
        source: SourceRef::id(format!("drill:{}", index)),
        method: EvidenceMethod::Fetched,
        observed_on: "2026-01-01".into(),
        note: Some(high_entropy_note(index, NOTE_BYTES)),
    });
    school
}

fn digest_for(records: &[CanonicalSchool]) -> String {
    let mut h = Sha256::new();
    for r in records {
        h.update(serde_json::to_vec(r).unwrap());
    }
    bytes_hex(&h.finalize())
}

fn is_enospc(err: &StoreError) -> bool {
    let mut current: &dyn std::error::Error = err;
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

fn baseline(store: &Store) {
    let schools: Vec<CanonicalSchool> = (0..BASELINE_COUNT).map(mk_school).collect();
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &schools).unwrap();
    let op = "baseline";
    let d = digest_for(&schools);
    batch.commit_once(op, &d).unwrap();
    let count = store.scan::<CanonicalSchool>(Table::Schools).unwrap().len();
    assert_eq!(count, BASELINE_COUNT, "baseline count: got {} expected {}", count, BASELINE_COUNT);
    println!("PASS: baseline {} schools", BASELINE_COUNT);
}

fn drain_writes(store: &Store) -> (Vec<(String, u64)>, Option<(usize, String, bool)>) {
    let mut committed: Vec<(String, u64)> = Vec::new();
    let mut first_failure = None;

    for attempt in 0..MAX_ATTEMPTS {
        let schools: Vec<CanonicalSchool> = ((BASELINE_COUNT + attempt * BATCH_COUNT)..(BASELINE_COUNT + (attempt + 1) * BATCH_COUNT))
            .map(mk_school)
            .collect();
        let mut batch = store.write_batch();
        batch.append_many(Table::Schools, &schools).unwrap();
        let op = format!("batch_{}", attempt);
        let d = digest_for(&schools);
        match batch.commit_once(&op, &d) {
            Ok(Application::Written(receipt)) => {
                committed.push((op, receipt.appended));
                if (attempt + 1) % 10 == 0 || attempt == MAX_ATTEMPTS - 1 {
                    println!("committed {} batches so far", attempt + 1);
                }
            }
            Ok(Application::Repeated(_)) => {}
            Err(e) => {
                if first_failure.is_none() {
                    let has_enospc = is_enospc(&e);
                    first_failure = Some((attempt, e.to_string(), has_enospc));
                    break;
                }
            }
        }
    }

    assert!(first_failure.is_some(),
        "no failure after {} attempts, committed: {}",
        MAX_ATTEMPTS, committed.len());
    (committed, first_failure)
}

fn verify_reopen(root: &Path, committed: &[(String, u64)], fail: Option<(usize, String, bool)>) {
    let _ = fs::remove_dir_all(root.join(".enospc_scratch"));
    drop(Store::open(root).unwrap());
    let store = Store::open(root).unwrap();

    let all_rows: Vec<CanonicalSchool> = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    let total_expected: u64 = BASELINE_COUNT as u64 + committed.iter().map(|(_, c)| *c).sum::<u64>();
    assert_eq!(all_rows.len(), total_expected as usize,
        "reopen count: got {} expected {}", all_rows.len(), total_expected);

    let ids: HashSet<String> = all_rows.iter().map(|s| s.id.as_str().to_string()).collect();

    for i in 0..BASELINE_COUNT {
        let expected_id = mk_school(i).id.as_str().to_string();
        assert!(ids.contains(&expected_id), "baseline {} missing", i);
    }

    let mut expected_start = BASELINE_COUNT;
    for (_, count) in committed {
        for j in 0..*count as usize {
            let expected_id = mk_school(expected_start + j).id.as_str().to_string();
            assert!(ids.contains(&expected_id), "committed {} missing", expected_id);
        }
        expected_start += *count as usize;
    }

    if let Some((fail_idx, ref err_str, was_enospc)) = fail {
        let fail_start = BASELINE_COUNT + fail_idx * BATCH_COUNT;
        for j in 0..BATCH_COUNT {
            let expected_id = mk_school(fail_start + j).id.as_str().to_string();
            assert!(!ids.contains(&expected_id), "failed {} leaked", j);
        }
        if was_enospc {
            println!("PASS: {} baseline, {} batches committed, {} refused (ENOSPC), no duplicates",
                BASELINE_COUNT, committed.len(), fail_idx);
        } else {
            println!("PASS: {} baseline, {} batches committed, {} refused (non-ENOSPC: {}), no duplicates",
                BASELINE_COUNT, committed.len(), fail_idx, err_str);
        }
    } else {
        println!("PASS: {} baseline, {} batches committed, no failure",
            BASELINE_COUNT, committed.len());
    }
}

fn main() {
    let root: PathBuf = std::env::args()
        .nth(1)
        .expect("usage: enospc <tmpfs-root>")
        .into();
    let tmp_name = root.join(".enospc_scratch");

    check_bounded_fs(&root);
    fs::create_dir_all(&tmp_name).unwrap();

    let store = Store::open(&root).unwrap();
    baseline(&store);
    let (committed, fail) = drain_writes(&store);
    drop(store);

    verify_reopen(&root, &committed, fail);
}
