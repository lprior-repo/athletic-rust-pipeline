use super::super::{
    admission::Node, invalid, reserve, AuthorityTableV2, CaptureBindingV2, ObjectRefV2,
    RawBlockReferenceV2, RawGenerationSealV2, RawPageBlockV2, RawPagePositionV2,
    RawPageReferenceV2, RawPageV2, RawPhysicalRowV2, RawTableManifestV2, RawTableReferenceV2,
    TableBindingV2, MAX_PAGE_RAW_BYTES, MAX_PAGE_ROWS, MAX_RAW_VALUE_BYTES, MAX_REFERENCE_FANOUT,
    MAX_TABLE_PAGES,
};
use crate::{Entity, ReadBudget, Store, StoreError, StoreResult, StoreSnapshot};
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, RelayResult, ReviewCase, ReviewVerdictRecord,
    SourceObservation,
};

pub(super) enum Input<'a, 's> {
    Held(&'a StoreSnapshot<'s>),
    Current(&'a Store),
}

impl Input<'_, '_> {
    fn read<R>(
        &self,
        original: &CaptureBindingV2,
        read: impl FnOnce(&StoreSnapshot<'_>) -> StoreResult<R>,
    ) -> StoreResult<R> {
        match self {
            Self::Held(snapshot) => read(snapshot),
            Self::Current(store) => {
                let _guard = store.lock_appends();
                let snapshot = store.snapshot_locked();
                super::binding::require_inputs(
                    original,
                    &super::binding::capture(store, &snapshot)?,
                )?;
                read(&snapshot)
            }
        }
    }
}

pub(super) fn walk(
    input: Input<'_, '_>,
    binding: &CaptureBindingV2,
    mut visit: impl FnMut(Node<'_>) -> StoreResult<ObjectRefV2>,
) -> StoreResult<RawGenerationSealV2> {
    let mut tables = reserve(6)?;
    for table in AuthorityTableV2::ALL {
        let table_binding = binding.table_binding(table)?;
        let reference = match table {
            AuthorityTableV2::Athletes => {
                scan::<CanonicalAthlete>(&input, binding, &table_binding, &mut visit)
            }
            AuthorityTableV2::SourceObservations => {
                scan::<SourceObservation>(&input, binding, &table_binding, &mut visit)
            }
            AuthorityTableV2::ReviewCases => {
                scan::<ReviewCase>(&input, binding, &table_binding, &mut visit)
            }
            AuthorityTableV2::IdentityVerdicts => {
                scan::<ReviewVerdictRecord>(&input, binding, &table_binding, &mut visit)
            }
            AuthorityTableV2::Performances => {
                scan::<CanonicalPerformance>(&input, binding, &table_binding, &mut visit)
            }
            AuthorityTableV2::RelayResults => {
                scan::<RelayResult>(&input, binding, &table_binding, &mut visit)
            }
        }?;
        tables.push(reference);
    }
    RawGenerationSealV2::new(
        binding.clone(),
        tables
            .try_into()
            .map_err(|_| invalid("six capture tables"))?,
    )
}

fn scan<T: Entity>(
    input: &Input<'_, '_>,
    capture: &CaptureBindingV2,
    binding: &TableBindingV2,
    visit: &mut impl FnMut(Node<'_>) -> StoreResult<ObjectRefV2>,
) -> StoreResult<RawTableReferenceV2> {
    let mut cursor = None;
    let mut directory = Directory::new()?;
    let mut ordinal = 0_u64;
    for index in 0..MAX_TABLE_PAGES {
        let mut rows = reserve(usize::from(MAX_PAGE_ROWS))?;
        let (_, next) = input.read(capture, |snapshot| {
            snapshot.scan_physical_observation_page_from_key::<T>(
                binding.table().table(),
                ReadBudget {
                    rows: u64::from(MAX_PAGE_ROWS),
                    bytes: u64::from(MAX_PAGE_RAW_BYTES),
                    record_bytes: MAX_RAW_VALUE_BYTES,
                },
                cursor.as_deref(),
                |_, key, value, _| {
                    rows.push(RawPhysicalRowV2::new(binding, key, value)?);
                    Ok(())
                },
            )
        })?;
        if !rows.is_empty() {
            let count = u64::try_from(rows.len()).map_err(|_| StoreError::CounterOverflow)?;
            let page = RawPageV2::new(binding, RawPagePositionV2::new(index, ordinal)?, rows)?;
            let reference = page.reference(visit(Node::Page(&page))?)?;
            directory.push(binding, reference, visit)?;
            ordinal = ordinal
                .checked_add(count)
                .ok_or(StoreError::CounterOverflow)?;
        }
        match next {
            Some(next) => cursor = Some(next),
            None => return directory.finish(binding, visit),
        }
    }
    Err(invalid("capture page ceiling reached before exhaustion"))
}

pub(super) struct Directory {
    pages: Vec<RawPageReferenceV2>,
    blocks: Vec<RawBlockReferenceV2>,
}

impl Directory {
    pub(super) fn new() -> StoreResult<Self> {
        Ok(Self {
            pages: reserve(usize::from(MAX_REFERENCE_FANOUT))?,
            blocks: reserve(usize::from(MAX_REFERENCE_FANOUT))?,
        })
    }

    pub(super) fn push(
        &mut self,
        binding: &TableBindingV2,
        page: RawPageReferenceV2,
        visit: &mut impl FnMut(Node<'_>) -> StoreResult<ObjectRefV2>,
    ) -> StoreResult<()> {
        self.pages.push(page);
        if self.pages.len() == usize::from(MAX_REFERENCE_FANOUT) {
            self.flush(binding, visit)?;
        }
        Ok(())
    }

    fn flush(
        &mut self,
        binding: &TableBindingV2,
        visit: &mut impl FnMut(Node<'_>) -> StoreResult<ObjectRefV2>,
    ) -> StoreResult<()> {
        if self.pages.is_empty() {
            return Ok(());
        }
        let index = u16::try_from(self.blocks.len()).map_err(|_| StoreError::CounterOverflow)?;
        let pages = std::mem::replace(&mut self.pages, reserve(usize::from(MAX_REFERENCE_FANOUT))?);
        let block = RawPageBlockV2::new(binding, index, pages)?;
        let reference = block.reference(visit(Node::Block(&block))?)?;
        self.blocks.push(reference);
        Ok(())
    }

    pub(super) fn finish(
        mut self,
        binding: &TableBindingV2,
        visit: &mut impl FnMut(Node<'_>) -> StoreResult<ObjectRefV2>,
    ) -> StoreResult<RawTableReferenceV2> {
        self.flush(binding, visit)?;
        let table = RawTableManifestV2::new(binding, self.blocks)?;
        table.reference(visit(Node::Table(&table))?)
    }
}
