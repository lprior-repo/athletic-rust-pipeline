use super::super::limits::ObjectLimits;
use super::*;
use crate::public_identity::retained::Live;

impl private::Sealed for Live<'_> {}

impl RawObjectSource for Live<'_> {
    fn require_capture(&self, binding: &CaptureBindingV2) -> StoreResult<()> {
        match crate::lineage::read(self.snapshot, self.journal)? {
            crate::lineage::RetainedIdentity::Present(identity) if identity == binding.0.store => {
                Ok(())
            }
            _ => Err(super::super::invalid(
                "retained capture belongs to another store",
            )),
        }
    }

    fn read_seal(
        &self,
        object: &ObjectRefV2,
        binding: &CaptureBindingV2,
    ) -> StoreResult<RawGenerationSealV2> {
        RawGenerationSealV2::decode_retained(
            &open(self.snapshot, self.journal, object, ObjectLimits::SEAL)?,
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
            &open(self.snapshot, self.journal, object, ObjectLimits::DIRECTORY)?,
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
            &open(self.snapshot, self.journal, object, ObjectLimits::DIRECTORY)?,
            object,
            binding,
        )
    }
    fn read_page(&self, object: &ObjectRefV2, binding: &TableBindingV2) -> StoreResult<RawPageV2> {
        RawPageV2::decode_retained(
            &open(self.snapshot, self.journal, object, ObjectLimits::PAGE)?,
            object,
            binding,
        )
    }
}

fn open<'a>(
    snapshot: &fjall::Snapshot,
    journal: &'a fjall::Keyspace,
    object: &ObjectRefV2,
    limits: ObjectLimits,
) -> StoreResult<crate::read::JournalObject<'a>> {
    crate::read::read_context_object_bounded(
        snapshot,
        journal,
        object.root().as_str(),
        super::physical::limits(object, limits)?,
    )
}

pub(crate) fn graph_physical_rows(
    snapshot: &fjall::Snapshot,
    journal: &fjall::Keyspace,
    object: &ObjectRefV2,
    visit: impl FnMut(&[u8], &[u8]) -> StoreResult<()>,
) -> StoreResult<()> {
    let reader = open(snapshot, journal, object, ObjectLimits::PAGE)?;
    super::physical::check_manifest(&reader, object)?;
    reader.for_each_physical_row(object.physical_bytes(), visit)
}
