use super::*;
use crate::public_identity::retained::{GraphReader, Read};

impl<R: Read, S: RawObjectSource> private::Sealed for GraphReader<'_, R, S> {}

impl<R: Read, S: RawObjectSource> RawObjectSource for GraphReader<'_, R, S> {
    fn require_capture(&self, binding: &CaptureBindingV2) -> StoreResult<()> {
        self.source.require_capture(binding)
    }
    fn read_seal(
        &self,
        object: &ObjectRefV2,
        binding: &CaptureBindingV2,
    ) -> StoreResult<RawGenerationSealV2> {
        self.source.read_seal(object, binding)
    }
    fn read_table(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawTableManifestV2> {
        self.source.read_table(object, binding)
    }
    fn read_block(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawPageBlockV2> {
        self.source.read_block(object, binding)
    }
    fn read_page(&self, object: &ObjectRefV2, binding: &TableBindingV2) -> StoreResult<RawPageV2> {
        self.source.read_page(object, binding)
    }
}
