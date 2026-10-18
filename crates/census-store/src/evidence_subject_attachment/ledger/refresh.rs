use super::*;

pub(super) fn refresh(
    ledger: &mut Ledger<'_>,
    store: &crate::Store,
    view: &StoreSnapshot<'_>,
    identity: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<bool> {
    let (physical, keys) = read::inventory(view, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if !ledger
        .physical
        .fingerprint()
        .same_contents(store, physical.fingerprint())?
    {
        return Ok(false);
    }
    grant.charge_steps(1).map_err(Error::Domain)?;
    if keys.len() != ledger.records.len()
        || ledger.records.len() != ledger.introductions.len()
        || ledger.records.len() != ledger.input_refs.len()
        || ledger.records.len() != ledger.receipts.len()
    {
        return Err(Error::Corrupt("retained ledger positional lengths").into());
    }
    verify_retained_materials(ledger, view, grant)?;
    let receipts = receipts::load(view, &ledger.records, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if receipts.len() != ledger.receipts.len() {
        return Err(Error::Corrupt("retained ledger receipt lengths").into());
    }
    for (current, retained) in receipts.iter().zip(&ledger.receipts) {
        grant.charge_steps(1).map_err(Error::Domain)?;
        if current != retained {
            return Err(Error::Corrupt("retained ledger receipt changed").into());
        }
    }
    ledger.receipts = receipts;
    ledger.validate_projection(identity, grant)?;
    Ok(true)
}

fn verify_retained_materials(
    ledger: &Ledger<'_>,
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for introduced in &ledger.introductions {
        grant.charge_steps(1).map_err(Error::Domain)?;
        context::verify_materials(view, introduced, grant)?;
    }
    Ok(())
}
