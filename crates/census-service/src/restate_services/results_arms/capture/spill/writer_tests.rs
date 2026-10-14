use super::*;
use crate::restate_services::results_arms::capture_tests::meet;
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::os::unix::fs::FileExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn tiny_fragments_stay_in_scratch_until_flush_and_empty_flush_is_free() -> TestResult {
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let lease = admission.claim()?;
    let mut directory = owner::Directory::create(root.path(), Arc::clone(&admission))?;
    let mut writer = Writer::new(
        &mut directory,
        UsJurisdiction::Wisconsin,
        Arc::clone(&admission),
    )?;
    let bytes = b"escaped \\\" \\u0000\n";
    bytes
        .chunks(1)
        .try_for_each(|fragment| writer.append(fragment))?;
    check!(eq; writer.buffer, bytes);
    check!(eq; admission.reserved_bytes()?, 0);
    check!(eq; admission.writes(), (0, 0));
    check!(eq; writer.file.metadata()?.len(), 0);
    check!(eq; admission.peak_scratch(), u64::try_from(writer.buffer.capacity())?);
    writer.flush_buffer()?;
    let count = u64::try_from(bytes.len())?;
    check!(eq; admission.reserved_bytes()?, count);
    check!(eq; admission.writes(), (1, count));
    check!(eq; std::fs::read(&writer.path)?, bytes);
    check!(eq; writer.bytes, count);
    check!(eq; <[u8; 32]>::from(writer.digest.clone().finalize()), <[u8; 32]>::from(Sha256::digest(bytes)));
    check!(eq; writer.buffer.len(), 0);
    writer.flush_buffer()?;
    check!(eq; admission.reserved_bytes()?, count);
    check!(eq; admission.writes(), (1, count));
    drop(writer);
    directory.unlink()?;
    drop(lease);
    check!(eq; admission.reserved_bytes()?, 0);
    Ok(())
}

#[test]
fn full_buffer_and_final_tail_keep_exact_bytes_digest_and_reservation() -> TestResult {
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let lease = admission.claim()?;
    let mut directory = owner::Directory::create(root.path(), Arc::clone(&admission))?;
    let mut writer = Writer::new(
        &mut directory,
        UsJurisdiction::Wisconsin,
        Arc::clone(&admission),
    )?;
    let capacity = writer.buffer.capacity();
    (0..capacity).try_for_each(|_| writer.append(b"x"))?;
    check!(eq; admission.writes(), (0, 0));
    writer.append(b"y")?;
    check!(eq; admission.reserved_bytes()?, u64::try_from(capacity)?);
    check!(eq; writer.buffer, b"y");
    let partition = writer.finish()?;
    let mut expected = vec![b'x'; capacity];
    expected.push(b'y');
    let count = u64::try_from(expected.len())?;
    check!(eq; std::fs::read(&partition.path)?, expected);
    check!(eq; partition.bytes, count);
    check!(eq; partition.digest, <[u8; 32]>::from(Sha256::digest(&expected)));
    check!(eq; admission.reserved_bytes()?, count);
    check!(eq; admission.writes(), (2, count));
    drop(partition);
    directory.unlink()?;
    drop(lease);
    check!(eq; admission.reserved_bytes()?, 0);
    Ok(())
}

#[test]
fn escaped_multibuffer_rows_keep_exact_serde_bytes_and_round_trip() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut first = meet(UsJurisdiction::Wisconsin, "a\0z", Some("malformed"));
    first.name = "\"\\\n\0é中".repeat(16_384);
    let second = meet(UsJurisdiction::Wisconsin, "a/z", None);
    let rows = vec![first, second];
    let mut expected = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        serde_json::to_writer(
            &mut expected,
            &wire::Borrowed::new(row, u64::try_from(index)? + 1)?,
        )?;
        expected.push(b'\n');
    }
    let expected_rows = rows.clone();
    let (dataset, admission) = super::super::tests::build(root.path(), rows)?;
    let partition = dataset
        .partitions
        .iter()
        .find(|partition| partition.jurisdiction == UsJurisdiction::Wisconsin)
        .ok_or("WI partition")?;
    let mut actual = vec![0; expected.len()];
    partition.file.read_exact_at(&mut actual, 0)?;
    check!(actual == expected, "encoded spill bytes differ");
    check!(eq; partition.digest, <[u8; 32]>::from(Sha256::digest(&expected)));
    check!(eq; dataset.disk_bytes, u64::try_from(expected.len())?);
    check!(eq; admission.reserved_bytes()?, dataset.disk_bytes);
    check!(eq; admission.writes().1, dataset.disk_bytes);
    check!(eq; super::super::tests::collect(&dataset)?, expected_rows);
    check!(!root.path().join(".results-source-spill").exists());
    Ok(())
}

#[test]
fn exact_disk_remainder_is_admitted_before_any_write() -> TestResult {
    for remaining in [2, 3] {
        let admission = Arc::new(SpillAdmission::default());
        let lease = admission.claim()?;
        admission.charge(budget::CAPTURE_BYTES - remaining)?;
        let mut output = Script::new(Arc::clone(&admission), [Step::Bytes(3)]);
        let result = write_buffer(&mut output, Path::new("fixture"), b"abc", &admission);
        if remaining == 3 {
            result?;
            check!(eq; output.bytes, b"abc");
            check!(eq; output.reservations, vec![budget::CAPTURE_BYTES]);
            check!(eq; admission.writes(), (1, 3));
            check!(eq; admission.reserved_bytes()?, budget::CAPTURE_BYTES);
        } else {
            check!(matches!(
                result,
                Err(StoreError::BudgetExceeded {
                    what: "results spill bytes",
                    needed: 13_958_643_713,
                    available: 13_958_643_712,
                })
            ));
            check!(eq; output.bytes.len(), 0);
            check!(eq; output.reservations.len(), 0);
            check!(eq; admission.writes(), (0, 0));
            check!(eq; admission.reserved_bytes()?, budget::CAPTURE_BYTES - 2);
        }
        drop(lease);
        check!(eq; admission.reserved_bytes()?, 0);
    }
    Ok(())
}

#[test]
fn interrupted_and_short_writes_charge_once_and_preserve_offsets() -> TestResult {
    let admission = Arc::new(SpillAdmission::default());
    let lease = admission.claim()?;
    let mut output = Script::new(
        Arc::clone(&admission),
        [
            Step::Bytes(2),
            Step::Error(ErrorKind::Interrupted),
            Step::Bytes(3),
        ],
    );
    write_buffer(&mut output, Path::new("fixture"), b"abcde", &admission)?;
    check!(eq; output.bytes, b"abcde");
    check!(eq; output.reservations, vec![5, 5, 5]);
    check!(eq; admission.writes(), (3, 5));
    check!(eq; admission.reserved_bytes()?, 5);
    drop(lease);
    check!(eq; admission.reserved_bytes()?, 0);
    Ok(())
}

#[test]
fn partial_failure_and_zero_progress_keep_entire_flush_charged_when_poisoned() -> TestResult {
    for last in [Step::Error(ErrorKind::PermissionDenied), Step::Bytes(0)] {
        let admission = Arc::new(SpillAdmission::default());
        let lease = admission.claim()?;
        let expected_io = matches!(last, Step::Error(_));
        let mut output = Script::new(Arc::clone(&admission), [Step::Bytes(2), last]);
        let error = write_buffer(&mut output, Path::new("fixture"), b"abcde", &admission)
            .err()
            .ok_or("failed flush accepted")?;
        if expected_io {
            check!(matches!(error, StoreError::Io { path, source }
                if path == Path::new("fixture") && source.kind() == ErrorKind::PermissionDenied));
        } else {
            check!(matches!(error, StoreError::Refused { detail }
                if detail == "results spill write made no progress"));
        }
        check!(eq; output.bytes, b"ab");
        check!(eq; output.reservations, vec![5, 5]);
        check!(eq; admission.writes(), (2, 2));
        check!(eq; admission.reserved_bytes()?, 5);
        admission.poison()?;
        drop(lease);
        check!(eq; admission.reserved_bytes()?, 5);
        check!(
            matches!(admission.claim(), Err(StoreError::Refused { detail })
            if detail == "results spill cleanup unresolved; 5 disk bytes remain reserved")
        );
    }
    Ok(())
}

#[test]
fn record_limit_and_total_overflow_refuse_before_io() -> TestResult {
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let lease = admission.claim()?;
    let mut directory = owner::Directory::create(root.path(), Arc::clone(&admission))?;
    let mut writer = Writer::new(
        &mut directory,
        UsJurisdiction::Wisconsin,
        Arc::clone(&admission),
    )?;
    writer.record_bytes = budget::SPILL_RECORD_BYTES - 1;
    writer.append(b"x")?;
    check!(eq; writer.record_bytes, budget::SPILL_RECORD_BYTES);
    check!(
        matches!(writer.append(b"y"), Err(StoreError::RowTooLarge { bytes, max })
        if bytes == budget::SPILL_RECORD_BYTES + 1 && max == budget::SPILL_RECORD_BYTES)
    );
    check!(eq; writer.buffer, b"x");
    writer.bytes = u64::MAX;
    check!(matches!(
        writer.flush_buffer(),
        Err(StoreError::CounterOverflow)
    ));
    check!(eq; admission.reserved_bytes()?, 0);
    check!(eq; admission.writes(), (0, 0));
    check!(eq; writer.file.metadata()?.len(), 0);
    drop(writer);
    directory.unlink()?;
    drop(lease);
    Ok(())
}

#[test]
fn final_flush_refusal_aborts_build_and_releases_after_verified_cleanup() -> TestResult {
    let root = tempfile::tempdir()?;
    let admission = Arc::new(SpillAdmission::default());
    let mut build = super::super::Build::new(root.path(), Arc::clone(&admission))?;
    admission.charge(budget::CAPTURE_BYTES)?;
    let row = meet(UsJurisdiction::Wisconsin, "1", None);
    let encoded = serde_json::to_vec(&wire::Borrowed::new(&row, 1)?)?;
    let bytes = u64::try_from(encoded.len())? + 1;
    let cancel = super::super::tests::uncancelled();
    build.push(row, &cancel)?;
    check!(eq; admission.writes(), (0, 0));
    let error = build
        .finish(&cancel)
        .err()
        .ok_or("over-budget finish published")?;
    check!(matches!(error, StoreError::BudgetExceeded {
        what: "results spill bytes", needed, available,
    } if needed == budget::CAPTURE_BYTES + bytes && available == budget::CAPTURE_BYTES));
    check!(eq; admission.writes(), (0, 0));
    check!(eq; admission.reserved_bytes()?, 0);
    check!(!root.path().join(".results-source-spill").exists());
    drop(admission.claim()?);
    Ok(())
}

enum Step {
    Bytes(usize),
    Error(ErrorKind),
}

struct Script {
    admission: Arc<SpillAdmission>,
    steps: VecDeque<Step>,
    bytes: Vec<u8>,
    reservations: Vec<u64>,
}

impl Script {
    fn new(admission: Arc<SpillAdmission>, steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            admission,
            steps: steps.into_iter().collect(),
            bytes: Vec::new(),
            reservations: Vec::new(),
        }
    }
}

impl Write for Script {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.reservations.push(
            self.admission
                .reserved_bytes()
                .map_err(std::io::Error::other)?,
        );
        match self.steps.pop_front() {
            Some(Step::Bytes(count)) => {
                let written = bytes
                    .get(..count)
                    .ok_or_else(|| std::io::Error::other("invalid script count"))?;
                self.bytes.extend_from_slice(written);
                Ok(count)
            }
            Some(Step::Error(kind)) => Err(std::io::Error::from(kind)),
            None => Err(std::io::Error::other("unexpected write")),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("unexpected flush"))
    }
}
