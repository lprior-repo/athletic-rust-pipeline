use super::super::{
    admission::Node, invalid, CaptureBindingV2, ObjectRefV2, RawGenerationSealV2, RawObjectSource,
    RawPageReferenceV2, RawPageV2, RawTableReferenceV2, TableBindingV2, MAX_PAGE_RAW_BYTES,
    MAX_PAGE_ROWS,
};
use crate::{StoreError, StoreResult};

pub(in crate::public_identity) fn verify(
    source: &impl RawObjectSource,
    binding: &CaptureBindingV2,
    object: &ObjectRefV2,
    mut visit: impl FnMut(Node<'_>, &ObjectRefV2) -> StoreResult<()>,
) -> StoreResult<RawGenerationSealV2> {
    source.require_capture(binding)?;
    let seal = source.read_seal(object, binding)?;
    for table in seal.tables() {
        verify_table(
            source,
            &binding.table_binding(table.table())?,
            table,
            &mut visit,
        )?;
    }
    visit(Node::Seal(&seal), object)?;
    Ok(seal)
}

pub(in crate::public_identity::paged_generation) fn verify_table(
    source: &impl RawObjectSource,
    binding: &TableBindingV2,
    reference: &RawTableReferenceV2,
    visit: &mut impl FnMut(Node<'_>, &ObjectRefV2) -> StoreResult<()>,
) -> StoreResult<()> {
    let table = source.read_table(&reference.object(), binding)?;
    reference.check_table(&table)?;
    let mut boundary: Option<Boundary> = None;
    for reference in table.blocks() {
        let block = source.read_block(&reference.object(), binding)?;
        reference.check_block(&block)?;
        for reference in block.pages() {
            let page = source.read_page(&reference.object(), binding)?;
            reference.check_page(&page)?;
            if let Some(previous) = &boundary {
                previous.check(&page)?;
            }
            boundary = Some(Boundary::new(&page, reference)?);
            visit(Node::Page(&page), &reference.object())?;
        }
        visit(Node::Block(&block), &reference.object())?;
    }
    visit(Node::Table(&table), &reference.object())?;
    Ok(())
}

struct Boundary {
    key: Vec<u8>,
    rows: u16,
    bytes: u32,
}

impl Boundary {
    fn new(page: &RawPageV2, reference: &RawPageReferenceV2) -> StoreResult<Self> {
        let last = page
            .rows()
            .last()
            .ok_or_else(|| invalid("empty retained capture page"))?;
        let mut key = super::super::reserve(last.physical_key().len())?;
        key.extend_from_slice(last.physical_key());
        Ok(Self {
            key,
            rows: reference.rows(),
            bytes: reference.raw_bytes(),
        })
    }

    fn check(&self, next: &RawPageV2) -> StoreResult<()> {
        let first = next
            .rows()
            .first()
            .ok_or_else(|| invalid("empty retained successor page"))?;
        let combined = self
            .bytes
            .checked_add(first.raw_bytes()?)
            .ok_or(StoreError::CounterOverflow)?;
        if self.key.as_slice() >= first.physical_key()
            || (self.rows < MAX_PAGE_ROWS && combined <= MAX_PAGE_RAW_BYTES)
        {
            return Err(invalid("retained page ordering or greedy boundary"));
        }
        Ok(())
    }
}
