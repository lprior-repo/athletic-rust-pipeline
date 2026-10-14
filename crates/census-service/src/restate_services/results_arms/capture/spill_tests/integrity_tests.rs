use super::*;
use sha2::{Digest, Sha256};
use std::io::Write;

#[test]
fn sealed_descriptors_are_read_only_and_digest_failure_releases_provisional_selection() -> TestResult
{
    let root = tempfile::tempdir()?;
    let (mut dataset, _) = build(
        root.path(),
        vec![meet(UsJurisdiction::Wisconsin, "1", None)],
    )?;
    let dataset_mut = Arc::get_mut(&mut dataset).ok_or("unique dataset")?;
    let partition = dataset_mut
        .partitions
        .iter_mut()
        .find(|partition| partition.jurisdiction == UsJurisdiction::Wisconsin)
        .ok_or("WI partition")?;
    check!((&partition.file).write(b"x").is_err());
    let first = partition.digest.first_mut().ok_or("digest byte")?;
    *first ^= 1;
    let admission = Arc::new(Admission::default());
    let error = selected(dataset, Arc::clone(&admission))
        .err()
        .ok_or("corrupt digest accepted")?;
    check!(error.to_string().contains("digest mismatch"));
    check!(eq; admission.selected_usage(), (0, 0));
    Ok(())
}

#[test]
fn structural_refusals_cover_truncation_trailing_bytes_count_framing_and_corrupt_json() -> TestResult
{
    let row = meet(UsJurisdiction::Wisconsin, "1", None);
    let raw = encoded(&[row])?;
    let cases = [
        (
            raw.get(..raw.len().checked_sub(1).ok_or("fixture length")?)
                .ok_or("fixture prefix")?
                .to_vec(),
            raw.len(),
            1,
            "truncated",
        ),
        (
            [raw.clone(), b"x".to_vec()].concat(),
            raw.len(),
            1,
            "trailing bytes",
        ),
        (raw.clone(), raw.len(), 2, "count, framing"),
        (raw.clone(), raw.len(), 0, "extra rows"),
        (b"{}\n".to_vec(), 3, 1, "corrupt"),
    ];
    cases
        .into_iter()
        .try_for_each(|(bytes, declared, rows, expected)| {
            let error = refused_partition(&bytes, declared, rows, Sha256::digest(&raw).into())?;
            check!(error.to_string().contains(expected));
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
    let without_newline = raw
        .get(..raw.len().checked_sub(1).ok_or("fixture length")?)
        .ok_or("fixture prefix")?;
    let error = refused_partition(
        without_newline,
        without_newline.len(),
        1,
        Sha256::digest(without_newline).into(),
    )?;
    check!(error.to_string().contains("framing"));
    Ok(())
}

#[test]
fn exact_publication_order_empty_id_and_jurisdiction_are_checked_without_new_locator_rules(
) -> TestResult {
    let first = meet(UsJurisdiction::Wisconsin, "1", None);
    let second = meet(UsJurisdiction::Wisconsin, "2", None);
    let mut empty = first.clone();
    empty.id.clear();
    let mut foreign = first.clone();
    foreign.jurisdiction = UsJurisdiction::Illinois;
    let ordered = encoded(&[first, second])?;
    let reordered = ordered
        .split_inclusive(|byte| *byte == b'\n')
        .rev()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let error = refused_partition(
        &reordered,
        reordered.len(),
        2,
        Sha256::digest(&reordered).into(),
    )?;
    check!(error.to_string().contains("merged publication order"));
    [vec![empty], vec![foreign]]
        .into_iter()
        .try_for_each(|rows| {
            let raw = encoded(&rows)?;
            let error = refused_partition(
                &raw,
                raw.len(),
                u64::try_from(rows.len())?,
                Sha256::digest(&raw).into(),
            )?;
            check!(error.to_string().contains("identity"));
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
    let root = tempfile::tempdir()?;
    let mut historical = meet(UsJurisdiction::Wisconsin, "raw id", Some("malformed date"));
    historical.results_url.clear();
    let (dataset, _) = build(root.path(), vec![historical])?;
    let selected = selected(dataset, Arc::new(Admission::default()))?;
    check!(eq; selected.pending, vec![String::new()]);
    Ok(())
}

#[test]
fn record_growth_is_refused_before_crossing_the_merged_ceiling() -> TestResult {
    let raw = vec![
        b'x';
        budget::SPILL_RECORD_BYTES
            .checked_add(1)
            .ok_or("fixture bytes")?
    ];
    let error = refused_partition(&raw, raw.len(), 1, Sha256::digest(&raw).into())?;
    check!(
        matches!(error, StoreError::RowTooLarge { max, .. } if max == budget::SPILL_RECORD_BYTES)
    );
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let mut build = Build::new(root.path(), Arc::clone(&admission))?;
    let mut row = meet(UsJurisdiction::Wisconsin, "1", None);
    row.name = "x".repeat(budget::SPILL_RECORD_BYTES);
    let error = build
        .push(row, &uncancelled())
        .err()
        .ok_or("oversized write accepted")?;
    let error = build.abort(error);
    check!(
        matches!(error, StoreError::RowTooLarge { max, .. } if max == budget::SPILL_RECORD_BYTES)
    );
    check!(eq; admission.reserved_bytes()?, 0);
    check!(!root.path().join(".results-source-spill").exists());
    Ok(())
}

#[test]
fn exact_disk_and_scratch_boundaries_do_not_release_poisoned_bytes() -> TestResult {
    let admission = Arc::new(SpillAdmission::default());
    let lease = admission.claim()?;
    admission.charge(budget::CAPTURE_BYTES)?;
    check!(matches!(
        admission.charge(1),
        Err(StoreError::BudgetExceeded {
            what: "results spill bytes",
            needed: 13_958_643_713,
            available: 13_958_643_712,
        })
    ));
    let mut scratch = budget::ScratchPermit::new(Arc::clone(&admission));
    scratch.reserve(budget::SPILL_SCRATCH_BYTES)?;
    check!(scratch.reserve(1).is_err());
    drop(scratch);
    admission.poison()?;
    drop(lease);
    check!(eq; admission.reserved_bytes()?, budget::CAPTURE_BYTES);
    check!(admission.claim().is_err());
    Ok(())
}

fn encoded(rows: &[SourceMeetRef]) -> TestResult<Vec<u8>> {
    rows.iter()
        .enumerate()
        .try_fold(Vec::new(), |mut bytes, (index, row)| {
            let position = u64::try_from(index)?
                .checked_add(1)
                .ok_or("position overflow")?;
            serde_json::to_writer(&mut bytes, &wire::Borrowed::new(row, position)?)?;
            bytes.push(b'\n');
            Ok(bytes)
        })
}

fn refused_partition(
    raw: &[u8],
    bytes: usize,
    rows: u64,
    digest: [u8; 32],
) -> TestResult<StoreError> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("wi");
    std::fs::write(&path, raw)?;
    let partition = Partition {
        file: File::open(&path)?,
        path,
        jurisdiction: UsJurisdiction::Wisconsin,
        bytes: u64::try_from(bytes)?,
        rows,
        digest,
    };
    let admission = Arc::new(SpillAdmission::default());
    match reader::visit(&partition, admission, &uncancelled(), |_, _| Ok(())) {
        Err(error) => Ok(error),
        Ok(_) => Err("invalid partition accepted".into()),
    }
}

#[test]
fn corrupt_original_capacity_charges_are_rejected_before_selection() -> TestResult {
    let row = meet(UsJurisdiction::Wisconsin, "1", None);
    let mut value = serde_json::to_value(wire::Borrowed::new(&row, 1)?)?;
    let charge = value
        .get_mut("charge")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("charge object")?;
    charge.insert("meet_heap".to_string(), serde_json::json!(0));
    let mut raw = serde_json::to_vec(&value)?;
    raw.push(b'\n');
    let error = refused_partition(&raw, raw.len(), 1, Sha256::digest(&raw).into())?;
    check!(error.to_string().contains("selection charge is corrupt"));
    Ok(())
}

#[test]
fn repeated_publication_ordinal_is_refused_independently_of_logical_id_history() -> TestResult {
    let rows = [
        meet(UsJurisdiction::Wisconsin, "1", None),
        meet(UsJurisdiction::Wisconsin, "2", None),
    ];
    let raw = encoded(&rows)?;
    let mut frames = raw
        .split_inclusive(|byte| *byte == b'\n')
        .map(serde_json::from_slice::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    let second = frames
        .get_mut(1)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("second frame")?;
    second.insert("position".to_string(), serde_json::json!(1));
    let repeated = frames.iter().try_fold(Vec::new(), |mut bytes, frame| {
        serde_json::to_writer(&mut bytes, frame)?;
        bytes.push(b'\n');
        Ok::<_, serde_json::Error>(bytes)
    })?;
    let error = refused_partition(
        &repeated,
        repeated.len(),
        2,
        Sha256::digest(&repeated).into(),
    )?;
    check!(error.to_string().contains("merged publication order"));
    Ok(())
}
