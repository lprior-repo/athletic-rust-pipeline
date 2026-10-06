mod reclaim;
mod stage;

pub use reclaim::Reclaimed;
pub use stage::{DerivedStage, Publication};

use super::{Store, StoreResult};

impl Store {
    pub fn stage_derived(&self) -> StoreResult<DerivedStage<'_>> {
        DerivedStage::begin(self)
    }

    pub fn reclaim_derived_generations(&self, budget: u64) -> StoreResult<Reclaimed> {
        reclaim::reclaim(self, budget)
    }
}
