use super::*;
use crate::acquisition::recovery::budget::io::RecoveryArchiveIo;

#[test]
fn prepared_recovery_io_diagnostics_share_original_cleanup_and_publication_cap() -> StoreResult<()>
{
    let mut parent = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    let mut account = parent.begin_target()?;
    let mut scratch = [0; 32768];
    account.reserve_scratch()?;
    account.diagnostic_bytes(512)?;
    {
        let mut io = RecoveryArchiveIo::new(&mut account, &mut scratch);
        io.diagnostic_bytes(4000)?;
        io.diagnostic_bytes(3680)?;
        exceeded(io.diagnostic_bytes(1), "diagnostic bytes");
    }
    account.start_publication()?;
    let mut io = RecoveryArchiveIo::new(&mut account, &mut scratch);
    exceeded(io.diagnostic_bytes(1), "diagnostic bytes");
    Ok(())
}
