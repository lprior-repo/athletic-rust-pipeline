use census_domain::model::RunManifest;

use crate::StoreResult;

use super::StoreSnapshot;

impl StoreSnapshot<'_> {
    pub fn run_manifest(&self) -> StoreResult<Option<RunManifest>> {
        crate::run::read(&self.snapshot, self.meta)
    }
}
