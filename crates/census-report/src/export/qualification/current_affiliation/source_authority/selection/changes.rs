use super::*;
use census_store::ReviewedRosterMemberWithdrawalV1;

pub(super) type Index<'a> = BTreeMap<(&'a str, u64), &'a ReviewedRosterMemberWithdrawalV1>;

pub(super) fn index<'a>(
    withdrawals: &'a [ReviewedRosterMemberWithdrawalV1],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Index<'a>> {
    let mut index = Index::new();
    for withdrawal in withdrawals {
        charge(grant, 512, 1)?;
        let key = (withdrawal.decision().member_claim(), withdrawal.ordinal());
        if index.insert(key, withdrawal).is_some() {
            return Err(invalid("duplicate reviewed member withdrawal ordinal"));
        }
    }
    Ok(index)
}

pub(super) fn for_assertions<'a>(
    assertions: &[LocallyCheckedAssertion<'a>],
    index: &Index<'a>,
    known: KnownBounds<'a>,
    athlete: &AthleteId,
    audit: &mut RetainedAudit,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<LocallyCheckedChange<'a>>> {
    let mut changes = Vec::new();
    for assertion in assertions {
        let raw = assertion.raw();
        let claim = raw.id.as_str();
        for (_, withdrawal) in index.range((claim, 0)..=(claim, u64::MAX)) {
            charge(grant, 0, 1)?;
            let change = checked(LocallyCheckedChange::check(RawChange {
                id: object(withdrawal.review_digest())?,
                target: raw.id,
                person: raw.person,
                kind: ChangeKind::WithdrawsClaim {
                    scope: withdrawal.decision().scope(),
                },
                evidence: raw.evidence,
                locator: raw.row_locator,
                reason_statement: raw.person_statement,
                known,
            }))?;
            reserve(&mut changes, grant)?;
            changes.push(change);
            audit.reference(athlete, withdrawal.review_digest())?;
            audit.reference(athlete, withdrawal.decision().member_claim())?;
        }
    }
    Ok(changes)
}

fn reserve(
    changes: &mut Vec<LocallyCheckedChange<'_>>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if changes.len() == changes.capacity() {
        let capacity = changes
            .capacity()
            .max(2)
            .checked_mul(2)
            .ok_or(StoreError::CounterOverflow)?;
        let added = capacity
            .checked_sub(changes.capacity())
            .ok_or(StoreError::CounterOverflow)?;
        let bytes = added
            .checked_mul(std::mem::size_of::<LocallyCheckedChange<'_>>())
            .ok_or(StoreError::CounterOverflow)?;
        charge(grant, bytes, 1)?;
        changes
            .try_reserve_exact(capacity - changes.len())
            .map_err(|_| invalid("reviewed member withdrawal allocation refused"))?;
    }
    Ok(())
}
