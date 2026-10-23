use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const RESERVE: u64 = 68_719_476_736;

fn sample(available_blocks: u64, fragment_bytes: u64) -> StatVfs {
    StatVfs {
        f_bsize: 4096,
        f_frsize: fragment_bytes,
        f_blocks: u64::MAX,
        f_bfree: u64::MAX,
        f_bavail: available_blocks,
        f_files: 0,
        f_ffree: 0,
        f_favail: 0,
        f_fsid: 0,
        f_flag: rustix::fs::StatVfsMountFlags::empty(),
        f_namemax: 255,
    }
}

#[test]
fn reserve_and_all_peak_components_have_exact_threshold() -> TestResult {
    let estimate = WriteSpaceEstimateV1::new(3, 5, 7)?;
    let required = 68_719_476_751;
    check!(eq; estimate.required_bytes(), required);
    let admitted = admit_available(&estimate, required)?;
    check!(eq; admitted.available_bytes(), required);
    check!(eq; admitted.required_bytes(), required);
    let short = required.checked_sub(1).ok_or("threshold underflow")?;
    check!(matches!(
        admit_available(&estimate, short),
        Err(StoreError::DiskBudgetExceeded { needed, available })
            if needed == required && available == short
    ));
    Ok(())
}

#[test]
fn zero_growth_still_requires_sixty_four_gib() -> TestResult {
    let estimate = WriteSpaceEstimateV1::new(0, 0, 0)?;
    check!(eq; estimate.required_bytes(), RESERVE);
    admit_available(&estimate, RESERVE)?;
    let short = RESERVE.checked_sub(1).ok_or("reserve underflow")?;
    check!(matches!(
        admit_available(&estimate, short),
        Err(StoreError::DiskBudgetExceeded { needed: RESERVE, available }) if available == short
    ));
    Ok(())
}

#[test]
fn estimate_overflow_refuses_each_sum_without_wrapping() -> TestResult {
    let maximum_growth = u64::MAX.checked_sub(RESERVE).ok_or("growth underflow")?;
    let exact = WriteSpaceEstimateV1::new(maximum_growth, 0, 0)?;
    check!(eq; exact.required_bytes(), u64::MAX);
    for (writes, temporary, compaction) in [
        (u64::MAX, 1, 0),
        (u64::MAX - 1, 1, 1),
        (maximum_growth, 0, 1),
        (0, maximum_growth, 1),
    ] {
        check!(matches!(
            WriteSpaceEstimateV1::new(writes, temporary, compaction),
            Err(StoreError::CounterExhausted)
        ));
    }
    Ok(())
}

#[test]
fn filesystem_probe_uses_available_blocks_and_fragment_size() -> TestResult {
    let path = Path::new("controlled-store/fjall");
    let estimate = WriteSpaceEstimateV1::new(0, 0, 0)?;
    let mut observed_path = None;
    let admitted = require_at_with(path, &estimate, |observed| {
        observed_path = Some(observed.to_owned());
        Ok(sample(16_777_216, 4096))
    })?;
    check!(eq; observed_path, Some(path.to_owned()));
    check!(eq; admitted.available_bytes(), RESERVE);
    check!(eq; admitted.required_bytes(), RESERVE);
    let short = RESERVE.checked_sub(1).ok_or("sample underflow")?;
    check!(matches!(
        require_at_with(path, &estimate, |_| Ok(sample(short, 1))),
        Err(StoreError::DiskBudgetExceeded { needed: RESERVE, available }) if available == short
    ));
    Ok(())
}

#[test]
fn filesystem_product_overflow_refuses_without_wrapping() -> TestResult {
    let path = Path::new("controlled-store/fjall");
    let estimate = WriteSpaceEstimateV1::new(0, 0, 0)?;
    check!(matches!(
        require_at_with(path, &estimate, |_| Ok(sample(u64::MAX, 2))),
        Err(StoreError::CounterExhausted)
    ));
    let admitted = require_at_with(path, &estimate, |_| Ok(sample(u64::MAX / 3, 3)))?;
    check!(eq; admitted.available_bytes(), u64::MAX);
    check!(eq; admitted.required_bytes(), RESERVE);
    Ok(())
}

#[test]
fn store_volume_probe_refuses_excess_estimate_without_a_write() -> TestResult {
    let (_directory, store) = crate::test_fixture::temp_store()?;
    let path = store.root().join(crate::DB_DIR);
    let volume = rustix::fs::statvfs(&path)?;
    let available = volume
        .f_bavail
        .checked_mul(volume.f_frsize)
        .ok_or("volume size")?;
    let estimate = WriteSpaceEstimateV1::new(available, 0, 0)?;
    let before = store.db.seqno();
    check!(
        matches!(store.require_write_space(&estimate), Err(StoreError::DiskBudgetExceeded { needed, .. }) if needed == estimate.required_bytes())
    );
    check!(eq; store.db.seqno(), before);
    Ok(())
}

#[test]
fn zero_fragment_size_and_probe_error_refuse_with_original_path() -> TestResult {
    let path = Path::new("controlled-store/fjall");
    let estimate = WriteSpaceEstimateV1::new(0, 0, 0)?;
    check!(matches!(
        require_at_with(path, &estimate, |_| Ok(sample(0, 0))),
        Err(StoreError::Io { path: observed, source })
            if observed == path && source.kind() == io::ErrorKind::InvalidData
    ));
    check!(matches!(
        require_at_with(path, &estimate, |_| Err(io::Error::from(io::ErrorKind::NotFound))),
        Err(StoreError::Io { path: observed, source })
            if observed == path && source.kind() == io::ErrorKind::NotFound
    ));
    Ok(())
}

#[test]
fn prior_admission_is_only_a_sample_and_later_probe_can_refuse() -> TestResult {
    let path = Path::new("controlled-store/fjall");
    let estimate = WriteSpaceEstimateV1::new(1024, 2048, 4096)?;
    let required = estimate.required_bytes();
    let first = require_at_with(path, &estimate, |_| Ok(sample(required, 1)))?;
    let short = required.checked_sub(1).ok_or("later sample underflow")?;
    check!(matches!(
        require_at_with(path, &estimate, |_| Ok(sample(short, 1))),
        Err(StoreError::DiskBudgetExceeded { needed, available })
            if needed == required && available == short
    ));
    check!(eq; first.required_bytes(), required);
    check!(eq; first.available_bytes(), required);
    Ok(())
}
