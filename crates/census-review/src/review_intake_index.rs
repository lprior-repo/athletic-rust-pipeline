use std::collections::HashMap;

use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, RetainedConflict, ReviewCase,
    ATHLETE_GENDER_UNVERIFIED_FAMILY, SCHOOL_ADDRESS_UNVERIFIED_FAMILY,
};
use census_store::{
    StoreError, StoreResult, StoreSnapshot, Table, MAX_ID_BYTES, MAX_ROWS_PER_TABLE,
};

pub(crate) struct CorpusIndex<'a, 's> {
    snapshot: &'a StoreSnapshot<'s>,
    conflicts: Option<Vec<(String, String)>>,
    quarantined: Option<Vec<ReviewCase>>,
}

impl<'a, 's> CorpusIndex<'a, 's> {
    pub(crate) fn new(snapshot: &'a StoreSnapshot<'s>) -> Self {
        Self {
            snapshot,
            conflicts: None,
            quarantined: None,
        }
    }

    pub(super) fn prepare(&mut self) -> StoreResult<()> {
        if self.conflicts.is_none() {
            self.conflicts = Some(read_conflicts(self.snapshot)?);
        }
        if self.quarantined.is_none() {
            self.quarantined = Some(read_quarantined(self.snapshot)?);
        }
        Ok(())
    }

    pub(super) fn conflicts_after(&self, after: Option<&str>) -> StoreResult<&[(String, String)]> {
        let conflicts = self
            .conflicts
            .as_deref()
            .ok_or_else(|| crate::consensus::invariant("conflict index disappeared"))?;
        let start = after.map_or(0, |after| {
            conflicts.partition_point(|(id, _)| id.as_str() <= after)
        });
        conflicts.get(start..).ok_or(StoreError::CounterOverflow)
    }

    pub(super) fn conflict_case(&self, id: &str) -> StoreResult<ReviewCase> {
        let conflict = self
            .snapshot
            .get_merged::<RetainedConflict>(Table::Conflicts, id)?
            .ok_or_else(|| {
                crate::consensus::invariant("indexed conflict disappeared from captured snapshot")
            })?;
        Ok(case(&conflict))
    }

    pub(super) fn quarantines_after(&self, after: Option<&str>) -> StoreResult<&[ReviewCase]> {
        let quarantined = self
            .quarantined
            .as_deref()
            .ok_or_else(|| crate::consensus::invariant("quarantine index disappeared"))?;
        let start = after.map_or(0, |after| {
            quarantined.partition_point(|case| case.id.as_str() <= after)
        });
        quarantined.get(start..).ok_or(StoreError::CounterOverflow)
    }
}

pub(super) fn case(conflict: &RetainedConflict) -> ReviewCase {
    ReviewCase::pending(
        &conflict.family,
        &conflict.subject_id,
        &conflict.subject,
        &conflict.detail,
    )
}

fn max_key_bytes() -> StoreResult<u64> {
    let key_bytes = u64::try_from(MAX_ID_BYTES).map_err(|_| StoreError::CounterOverflow)?;
    key_bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(128))
        .and_then(|bytes| bytes.checked_mul(MAX_ROWS_PER_TABLE))
        .ok_or(StoreError::CounterOverflow)
}

fn read_conflicts(snapshot: &StoreSnapshot<'_>) -> StoreResult<Vec<(String, String)>> {
    let mut rows = Vec::new();
    let mut bytes = 0_u64;
    let max = max_key_bytes()?;
    snapshot.for_each_merged(Table::Conflicts, |conflict: RetainedConflict| {
        #[cfg(test)]
        super::tests::decoded()?;
        let case = case(&conflict);
        let size = case
            .id
            .len()
            .checked_add(conflict.id.len())
            .ok_or(StoreError::CounterOverflow)?;
        bytes = bytes
            .checked_add(u64::try_from(size).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        if bytes > max {
            return Err(crate::consensus::invariant(
                "conflict index exceeds bounded key bytes",
            ));
        }
        rows.try_reserve(1).map_err(|error| {
            crate::consensus::invariant(format!("cannot reserve conflict index: {error}"))
        })?;
        rows.push((case.id, conflict.id));
        Ok(())
    })?;
    rows.sort_unstable();
    Ok(rows)
}

fn school_address_index(snapshot: &StoreSnapshot<'_>) -> StoreResult<HashMap<String, bool>> {
    let mut addressed = HashMap::new();
    snapshot.for_each_merged(Table::Schools, |school: CanonicalSchool| {
        #[cfg(test)]
        super::tests::decoded()?;
        addressed.insert(
            school.id.as_str().to_owned(),
            !school.postal_addresses.is_empty(),
        );
        Ok(())
    })?;
    Ok(addressed)
}

fn quarantine_cases(
    athlete: &CanonicalAthlete,
    addressed: &HashMap<String, bool>,
) -> [Option<ReviewCase>; 2] {
    let gender = (athlete.gender == Gender::Unknown).then(|| {
        ReviewCase::pending(
            ATHLETE_GENDER_UNVERIFIED_FAMILY,
            athlete.id.as_str(),
            athlete.canonical_name.clone(),
            "no source supplied a boys or girls competition category; retained gender is Unknown",
        )
    });
    let address = (!addressed
        .get(athlete.school.as_str())
        .is_some_and(|addressed| *addressed))
    .then(|| {
        ReviewCase::pending(
            SCHOOL_ADDRESS_UNVERIFIED_FAMILY,
            athlete.id.as_str(),
            athlete.canonical_name.clone(),
            "the athlete school carries no postal address claim; recruiting mail cannot be addressed",
        )
    });
    [gender, address]
}

fn read_quarantined(snapshot: &StoreSnapshot<'_>) -> StoreResult<Vec<ReviewCase>> {
    let addressed = school_address_index(snapshot)?;
    let mut rows = Vec::new();
    let mut bytes = 0_u64;
    let max = max_key_bytes()?;
    snapshot.for_each_merged(Table::Athletes, |athlete: CanonicalAthlete| {
        #[cfg(test)]
        super::tests::decoded()?;
        for case in quarantine_cases(&athlete, &addressed).into_iter().flatten() {
            charge(&mut bytes, max, &case, &mut rows)?;
        }
        Ok(())
    })?;
    rows.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    Ok(rows)
}

fn charge(
    bytes: &mut u64,
    max: u64,
    case: &ReviewCase,
    rows: &mut Vec<ReviewCase>,
) -> StoreResult<()> {
    let size = case
        .id
        .len()
        .checked_add(case.subject_id.len())
        .ok_or(StoreError::CounterOverflow)?;
    *bytes = bytes
        .checked_add(u64::try_from(size).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    if *bytes > max {
        return Err(crate::consensus::invariant(
            "quarantine index exceeds bounded key bytes",
        ));
    }
    rows.try_reserve(1).map_err(|error| {
        crate::consensus::invariant(format!("cannot reserve quarantine index: {error}"))
    })?;
    rows.push(case.clone());
    Ok(())
}
