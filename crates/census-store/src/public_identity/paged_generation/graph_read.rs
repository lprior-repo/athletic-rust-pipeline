use super::{
    admission::Node, capture::verify, invalid, AuthorityTableV2, CaptureBindingV2, ObjectRefV2,
    RawGenerationSealV2, RawObjectSource, RawPageV2, RawTableReferenceV2,
};
use crate::{Entity, ReadBudget, ReadCounts, StoreError, StoreResult};

mod scan;

pub(crate) struct VerifiedRawGraph<'a, S: RawObjectSource> {
    source: &'a S,
    seal: RawGenerationSealV2,
    #[cfg(test)]
    object: &'a ObjectRefV2,
}

impl<'a, S: RawObjectSource> VerifiedRawGraph<'a, S> {
    pub(crate) fn verify(
        source: &'a S,
        binding: &CaptureBindingV2,
        object: &'a ObjectRefV2,
    ) -> StoreResult<Self> {
        let seal = verify::verify(source, binding, object, |_, _| Ok(()))?;
        Ok(Self {
            source,
            seal,
            #[cfg(test)]
            object,
        })
    }

    pub(crate) fn binding(&self) -> &CaptureBindingV2 {
        self.seal.binding()
    }
    #[cfg(test)]
    pub(crate) fn seal(&self) -> &ObjectRefV2 {
        self.object
    }

    #[cfg(test)]
    pub(crate) fn table_inventory(
        &self,
        table: AuthorityTableV2,
    ) -> StoreResult<super::InventoryV2> {
        Ok(self.table_reference(table)?.inventory())
    }

    fn table_reference(&self, table: AuthorityTableV2) -> StoreResult<&RawTableReferenceV2> {
        self.seal
            .tables()
            .iter()
            .find(|reference| reference.table() == table)
            .ok_or_else(|| invalid("verified graph table is absent"))
    }

    pub(crate) fn for_each_physical<T: Entity>(
        &self,
        table: AuthorityTableV2,
        budget: ReadBudget,
        mut visit: impl FnMut(T, &[u8], &[u8], u64, u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        let reference = self.table_reference(table)?;
        scan::preflight(reference.inventory(), budget)?;
        let mut counts = ReadCounts::default();
        verify::verify_table(
            self.source,
            &self.binding().table_binding(table)?,
            reference,
            &mut |node, _| {
                if let Node::Page(page) = node {
                    scan::page(page, table, budget, &mut counts, &mut visit)?;
                }
                Ok(())
            },
        )?;
        if counts.decoded_rows != reference.inventory().rows()
            || counts.encoded_bytes != reference.inventory().raw_bytes()
        {
            return Err(invalid("verified graph scan inventory mismatch"));
        }
        Ok(counts)
    }

    pub(crate) fn for_each_observation<T: Entity>(
        &self,
        table: AuthorityTableV2,
        budget: ReadBudget,
        mut visit: impl FnMut(T, u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        self.for_each_physical(table, budget, |row, _, _, _, bytes| visit(row, bytes))
    }

    pub(crate) fn for_each_merged<T: Entity>(
        &self,
        table: AuthorityTableV2,
        budget: ReadBudget,
        mut visit: impl FnMut(T, u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        let mut current = None;
        let mut merged = 0;
        let mut counts = self.for_each_observation(table, budget, |row, bytes| {
            merged = crate::read::accept_merged_row(
                &mut current,
                row,
                &mut |row| visit(row, bytes),
                merged,
            )?;
            Ok(())
        })?;
        if let Some(row) = current {
            merged = crate::read::publish_merged_row(
                row,
                &mut |row| visit(row, counts.encoded_bytes),
                merged,
            )?;
        }
        counts.merged_rows = merged;
        Ok(counts)
    }
}
