mod forwarding;
mod frozen;
mod live;
mod physical;
pub(crate) use frozen::FrozenRawObjectSource;
pub(crate) use live::graph_physical_rows;

use super::{
    CaptureBindingV2, ObjectRefV2, RawGenerationSealV2, RawPageBlockV2, RawPageV2,
    RawTableManifestV2, TableBindingV2,
};
use crate::{StoreResult, StoreSnapshot};

mod private {
    pub trait Sealed {}
}

pub(crate) trait RawObjectSource: private::Sealed {
    fn require_capture(&self, binding: &CaptureBindingV2) -> StoreResult<()>;
    fn read_seal(
        &self,
        object: &ObjectRefV2,
        binding: &CaptureBindingV2,
    ) -> StoreResult<RawGenerationSealV2>;
    fn read_table(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawTableManifestV2>;
    fn read_block(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawPageBlockV2>;
    fn read_page(&self, object: &ObjectRefV2, binding: &TableBindingV2) -> StoreResult<RawPageV2>;
}

impl private::Sealed for StoreSnapshot<'_> {}

impl RawObjectSource for StoreSnapshot<'_> {
    fn require_capture(&self, binding: &CaptureBindingV2) -> StoreResult<()> {
        if self.store_identity()? != binding.0.store {
            return Err(super::invalid("retained capture belongs to another store"));
        }
        Ok(())
    }

    fn read_seal(
        &self,
        object: &ObjectRefV2,
        binding: &CaptureBindingV2,
    ) -> StoreResult<RawGenerationSealV2> {
        self.raw_generation_seal(object, binding)
    }

    fn read_table(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawTableManifestV2> {
        self.raw_generation_table(object, binding)
    }

    fn read_block(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawPageBlockV2> {
        self.raw_generation_block(object, binding)
    }

    fn read_page(&self, object: &ObjectRefV2, binding: &TableBindingV2) -> StoreResult<RawPageV2> {
        self.raw_generation_page(object, binding)
    }
}

#[cfg(test)]
mod tests;
