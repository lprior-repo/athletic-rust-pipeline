use super::*;
use crate::public_identity_tests::support::TestResult;

#[test]
fn frozen_byte_ceiling_accepts_exact_and_refuses_one_more_without_allocation() -> TestResult {
    check!(eq; MAX_FROZEN_PUBLIC_IDENTITY_BYTES, 68_719_476_736);
    bound(38_022_727_922, MAX_FROZEN_PUBLIC_IDENTITY_BYTES)?;
    bound(
        MAX_FROZEN_PUBLIC_IDENTITY_BYTES,
        MAX_FROZEN_PUBLIC_IDENTITY_BYTES,
    )?;
    let one_over = MAX_FROZEN_PUBLIC_IDENTITY_BYTES
        .checked_add(1)
        .ok_or("ceiling overflow")?;
    let error = bound(one_over, MAX_FROZEN_PUBLIC_IDENTITY_BYTES).err();
    check!(matches!(error, Some(StoreError::RowTooLarge { bytes, max })
        if bytes == one_over && max == MAX_FROZEN_PUBLIC_IDENTITY_BYTES));
    Ok(())
}
