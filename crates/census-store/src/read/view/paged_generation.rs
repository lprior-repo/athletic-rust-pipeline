use super::object_read::{read_context_object_bounded, ContextReadLimits};
use super::{JournalObject, StoreSnapshot};
use crate::public_identity::paged_generation::{
    CaptureBindingV2, ObjectRefV2, RawGenerationSealV2, RawPageBlockV2, RawPageV2,
    RawTableManifestV2, TableBindingV2, MAX_DIRECTORY_JSONL_BYTES, MAX_DIRECTORY_RECORDS,
    MAX_PAGE_JSONL_BYTES, MAX_PAGE_RECORDS, MAX_RECORD_BYTES, MAX_SEAL_JSONL_BYTES,
};
use crate::{StoreError, StoreResult};

impl<'s> StoreSnapshot<'s> {
    pub fn raw_generation_page(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawPageV2> {
        let reader = self.raw_generation_object(object, MAX_PAGE_JSONL_BYTES, MAX_PAGE_RECORDS)?;
        RawPageV2::decode_retained(&reader, object, binding)
    }

    pub fn raw_generation_block(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawPageBlockV2> {
        let reader =
            self.raw_generation_object(object, MAX_DIRECTORY_JSONL_BYTES, MAX_DIRECTORY_RECORDS)?;
        RawPageBlockV2::decode_retained(&reader, object, binding)
    }

    pub fn raw_generation_table(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawTableManifestV2> {
        let reader =
            self.raw_generation_object(object, MAX_DIRECTORY_JSONL_BYTES, MAX_DIRECTORY_RECORDS)?;
        RawTableManifestV2::decode_retained(&reader, object, binding)
    }

    pub fn raw_generation_seal(
        &self,
        object: &ObjectRefV2,
        binding: &CaptureBindingV2,
    ) -> StoreResult<RawGenerationSealV2> {
        let reader = self.raw_generation_object(object, MAX_SEAL_JSONL_BYTES, 1)?;
        RawGenerationSealV2::decode_retained(&reader, object, binding)
    }

    fn raw_generation_object(
        &self,
        object: &ObjectRefV2,
        bytes: u32,
        records: u32,
    ) -> StoreResult<JournalObject<'s>> {
        if object.bytes() > bytes || object.records() > records {
            return Err(StoreError::Refused {
                detail: "raw generation object exceeds kind limits".into(),
            });
        }
        read_context_object_bounded(
            &self.snapshot,
            self.journal,
            object.root().as_str(),
            ContextReadLimits {
                bytes: usize::try_from(object.bytes()).map_err(|_| StoreError::CounterOverflow)?,
                records: usize::try_from(object.records())
                    .map_err(|_| StoreError::CounterOverflow)?,
                record_bytes: MAX_RECORD_BYTES,
            },
        )
    }
}
