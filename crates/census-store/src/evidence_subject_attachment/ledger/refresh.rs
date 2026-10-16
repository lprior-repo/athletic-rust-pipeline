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
    contexts(ledger, view, grant)?;
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
    let inventory =
        domain::calculate_reviewed_reference_inventory_with_grant(&ledger.records, grant)
            .map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if inventory != ledger.inventory {
        return Err(Error::InventoryMoved.into());
    }
    ledger.validate_projection(identity, grant)?;
    for (record, inputs) in ledger.records.iter().zip(&ledger.input_refs) {
        fences::verify(ledger, record, inputs, grant)?;
        super::super::target::verify_historical(ledger, record, grant)?;
    }
    Ok(true)
}

fn contexts(
    ledger: &Ledger<'_>,
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for (index, record) in ledger.records.iter().enumerate() {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let prior = ledger
            .introductions
            .get(..index)
            .ok_or(Error::Corrupt("retained ledger prior prefix"))?;
        let introduced = ledger
            .introductions
            .get(index)
            .ok_or(Error::Corrupt("retained ledger context position"))?;
        context::accept(view, prior, introduced, record, grant)?;
    }
    Ok(())
}
