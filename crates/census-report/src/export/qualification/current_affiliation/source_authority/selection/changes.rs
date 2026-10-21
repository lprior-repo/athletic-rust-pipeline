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
            let mapped = mapped_person(raw.person, withdrawal.decision().person())?;
            audit.reference(athlete, withdrawal.review_digest())?;
            audit.reference(athlete, withdrawal.decision().member_claim())?;
            if !mapped {
                continue;
            }
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
        }
    }
    Ok(changes)
}

fn mapped_person(person: PersonCandidate<'_>, expected: &SourceSubject) -> StoreResult<bool> {
    match person {
        PersonCandidate::Mapped { person, .. } if person == expected => Ok(true),
        PersonCandidate::Unbound { person, .. } if person == expected => Ok(false),
        _ => Err(invalid(
            "reviewed member withdrawal person differs from assertion",
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::SourceNamespace;
    use std::num::NonZeroU64;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn unbound_withdrawal_requires_exact_source_person_before_skipping() -> TestResult {
        let source = SourceSubject::new(SourceNamespace::MilesplitAthlete, "77".to_owned())?;
        let foreign = SourceSubject::new(SourceNamespace::MilesplitAthlete, "78".to_owned())?;
        let wrong_namespace = SourceSubject::new(SourceNamespace::TfrrsAthlete, "77".to_owned())?;
        for gap in [SubjectGap::UnacceptedBinding, SubjectGap::ProviderIdReused] {
            let person = PersonCandidate::Unbound {
                person: &source,
                gap,
            };
            check!(!mapped_person(person, &source)?);
            check!(mapped_person(person, &foreign).is_err());
            check!(mapped_person(person, &wrong_namespace).is_err());
        }
        check!(mapped_person(PersonCandidate::MissingKey, &source).is_err());
        Ok(())
    }

    #[test]
    fn mapped_withdrawal_requires_exact_source_person() -> TestResult {
        let source = SourceSubject::new(SourceNamespace::MilesplitAthlete, "77".to_owned())?;
        let foreign = SourceSubject::new(SourceNamespace::MilesplitAthlete, "78".to_owned())?;
        let wrong_namespace = SourceSubject::new(SourceNamespace::TfrrsAthlete, "77".to_owned())?;
        let athlete = AthleteId::mint("ath_subject", &["controlled-withdrawal-person"]);
        let revision = NonZeroU64::new(1).ok_or("zero controlled revision")?;
        let known = KnownBounds::new(object("controlled-lineage")?, revision, revision)?;
        let person = PersonCandidate::Mapped {
            person: &source,
            athlete: &athlete,
            decision: object("controlled-person-binding")?,
            known,
        };
        check!(mapped_person(person, &source)?);
        check!(mapped_person(person, &foreign).is_err());
        check!(mapped_person(person, &wrong_namespace).is_err());
        Ok(())
    }
}
