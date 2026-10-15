use super::{invalid, DatedRosterMembership, StoreResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DatedMembership(Vec<DatedRosterMembership>);

impl DatedMembership {
    pub(super) fn new(proof: DatedRosterMembership) -> StoreResult<Self> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(1)
            .map_err(|_| invalid("dated roster membership allocation refused"))?;
        values.push(proof);
        Ok(Self(values))
    }

    pub(super) fn get(&self) -> StoreResult<&DatedRosterMembership> {
        match self.0.as_slice() {
            [proof] => Ok(proof),
            _ => Err(invalid("dated roster membership cardinality")),
        }
    }
}
