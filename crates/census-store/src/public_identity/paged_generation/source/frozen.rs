use super::super::limits::ObjectLimits;
use super::*;
use census_domain::model::StoreIdentity;
use std::collections::BTreeMap;

pub(crate) struct FrozenRawObjectSource<'a> {
    identity: &'a StoreIdentity,
    bindings: &'a [CaptureBindingV2],
    rows: &'a BTreeMap<Vec<u8>, Vec<u8>>,
}

impl private::Sealed for FrozenRawObjectSource<'_> {}

impl<'a> FrozenRawObjectSource<'a> {
    pub(crate) fn new(
        identity: &'a StoreIdentity,
        bindings: &'a [CaptureBindingV2],
        rows: &'a BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> StoreResult<Self> {
        if bindings.iter().any(|binding| &binding.0.store != identity) {
            return Err(super::super::invalid(
                "frozen capture belongs to another store",
            ));
        }
        Ok(Self {
            identity,
            bindings,
            rows,
        })
    }

    fn open(
        &self,
        object: &ObjectRefV2,
        limits: ObjectLimits,
    ) -> StoreResult<crate::read::JournalObject<'_>> {
        crate::read::read_frozen_context_object_bounded(
            self.rows,
            object.root().as_str(),
            super::physical::limits(object, limits)?,
        )
    }
}

impl RawObjectSource for FrozenRawObjectSource<'_> {
    fn require_capture(&self, binding: &CaptureBindingV2) -> StoreResult<()> {
        if &binding.0.store != self.identity || !self.bindings.contains(binding) {
            return Err(super::super::invalid("frozen source capture identity"));
        }
        Ok(())
    }
    fn read_seal(
        &self,
        object: &ObjectRefV2,
        binding: &CaptureBindingV2,
    ) -> StoreResult<RawGenerationSealV2> {
        RawGenerationSealV2::decode_retained(
            &self.open(object, ObjectLimits::SEAL)?,
            object,
            binding,
        )
    }
    fn read_table(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawTableManifestV2> {
        RawTableManifestV2::decode_retained(
            &self.open(object, ObjectLimits::DIRECTORY)?,
            object,
            binding,
        )
    }
    fn read_block(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawPageBlockV2> {
        RawPageBlockV2::decode_retained(
            &self.open(object, ObjectLimits::DIRECTORY)?,
            object,
            binding,
        )
    }
    fn read_page(&self, object: &ObjectRefV2, binding: &TableBindingV2) -> StoreResult<RawPageV2> {
        RawPageV2::decode_retained(&self.open(object, ObjectLimits::PAGE)?, object, binding)
    }
}
