use super::{ResultsPlanError, MAX_PLAN_ALLOCATIONS};

#[derive(Debug, Default, Clone, Copy)]
pub(in crate::restate_services) struct PlanAllocation {
    charged: u64,
}

impl PlanAllocation {
    pub(in crate::restate_services) fn charge(
        &mut self,
        bytes: usize,
    ) -> Result<(), ResultsPlanError> {
        let bytes = u64::try_from(bytes).map_err(|_| ResultsPlanError::Limit("allocation"))?;
        self.charge_u64(bytes)
    }

    pub(in crate::restate_services) fn charge_u64(
        &mut self,
        bytes: u64,
    ) -> Result<(), ResultsPlanError> {
        let charged = self
            .charged
            .checked_add(bytes)
            .filter(|sum| *sum <= MAX_PLAN_ALLOCATIONS)
            .ok_or(ResultsPlanError::Limit("allocation"))?;
        self.charged = charged;
        Ok(())
    }

    pub(in crate::restate_services) fn remaining(&self) -> Result<u64, ResultsPlanError> {
        MAX_PLAN_ALLOCATIONS
            .checked_sub(self.charged)
            .ok_or(ResultsPlanError::Limit("allocation"))
    }

    pub(in crate::restate_services) fn reserve<T>(
        &mut self,
        output: &mut Vec<T>,
        count: usize,
    ) -> Result<(), ResultsPlanError> {
        let bytes = count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(ResultsPlanError::Limit("allocation"))?;
        self.charge(bytes)?;
        output
            .try_reserve_exact(count)
            .map_err(|_| ResultsPlanError::Allocation)
    }
}
