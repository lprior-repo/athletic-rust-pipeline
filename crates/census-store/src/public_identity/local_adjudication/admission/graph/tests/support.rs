use super::*;
use census_domain::model::ReviewVerdictRecord;

pub(super) struct Generation {
    pub(super) capture: CaptureBindingV2,
    pub(super) page: RawPageV2,
    pub(super) block: RawPageBlockV2,
    pub(super) tables: Vec<RawTableManifestV2>,
    pub(super) seal: RawGenerationSealV2,
}

impl Generation {
    pub(super) fn new(store: &Store, padding: usize) -> TestResult<(Self, GraphPlan)> {
        let capture = CaptureBindingV2::new(
            store.store_identity()?,
            FormatBindingV2::new(1, 1, 1)?,
            RunBindingV2::Unbound,
            CapturePositionV2::new(9, 7, 12, [3, 4, 5, 6, 7, 8]),
        )?;
        let binding = capture.table_binding(AuthorityTableV2::IdentityVerdicts)?;
        let value = ReviewVerdictRecord {
            id: "verdict".into(),
            case_id: "case".into(),
            subject_id: "subject".into(),
            family: "identity".into(),
            kind: "rejected".into(),
            field: "identity".into(),
            value: "unknown".into(),
            accepted: false,
            confidence: 0,
            rationale: "deterministic graph admission fixture".into(),
            reviewer: "fixture".into(),
            observed_at: "2026-10-15".into(),
            member_ids: Vec::new(),
        };
        let key = crate::keys::observation_key(crate::Table::IdentityVerdicts, &value.id, 0);
        let mut raw = serde_json::to_vec(&value)?;
        raw.resize(raw.len().max(padding), b' ');
        let page = RawPageV2::new(
            &binding,
            RawPagePositionV2::new(0, 0)?,
            vec![RawPhysicalRowV2::new(&binding, &key, &raw)?],
        )?;
        let mut builder = GraphPlanBuilder::new(capture.clone());
        let page_ref = page.reference(builder.add(store, bridge::Node::Page(&page))?)?;
        let block = RawPageBlockV2::new(&binding, 0, vec![page_ref])?;
        let block_ref = block.reference(builder.add(store, bridge::Node::Block(&block))?)?;
        let mut tables = Vec::new();
        let mut references = Vec::new();
        for kind in AuthorityTableV2::ALL {
            let blocks = if kind == AuthorityTableV2::IdentityVerdicts {
                vec![block_ref.clone()]
            } else {
                Vec::new()
            };
            let table = RawTableManifestV2::new(&capture.table_binding(kind)?, blocks)?;
            references.push(table.reference(builder.add(store, bridge::Node::Table(&table))?)?);
            tables.push(table);
        }
        let references: [RawTableReferenceV2; 6] =
            references.try_into().map_err(|_| "six authority tables")?;
        let seal = RawGenerationSealV2::new(capture.clone(), references)?;
        let plan = builder.finish(store, &seal)?;
        Ok((
            Self {
                capture,
                page,
                block,
                tables,
                seal,
            },
            plan,
        ))
    }

    pub(super) fn plan(&self, store: &Store) -> StoreResult<GraphPlan> {
        let mut builder = GraphPlanBuilder::new(self.capture.clone());
        builder.add(store, bridge::Node::Page(&self.page))?;
        builder.add(store, bridge::Node::Block(&self.block))?;
        for table in &self.tables {
            builder.add(store, bridge::Node::Table(table))?;
        }
        builder.finish(store, &self.seal)
    }

    pub(super) fn stage(&self, store: &Store, reservation: &GraphReservation) -> StoreResult<()> {
        stage(store, reservation, bridge::Node::Page(&self.page))?;
        stage(store, reservation, bridge::Node::Block(&self.block))?;
        for table in &self.tables {
            stage(store, reservation, bridge::Node::Table(table))?;
        }
        stage(store, reservation, bridge::Node::Seal(&self.seal))
    }
}

pub(super) fn enrolled() -> TestResult<(tempfile::TempDir, Store, crate::PublicIdentityWriterV1)> {
    let (directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

pub(super) fn reserve(
    store: &Store,
    operation: &str,
    plan: GraphPlan,
) -> StoreResult<GraphReservation> {
    let _appends = store.lock_appends();
    reserve_graph(store, &RawGenerationOperationIdV2::new(operation)?, plan)
}

pub(super) fn stage(
    store: &Store,
    reservation: &GraphReservation,
    node: bridge::Node<'_>,
) -> StoreResult<()> {
    let measured = node.measure(store)?;
    let permit = reservation.permit_for(store, measured.root())?;
    node.stage(store, &permit)
}

pub(super) fn counters(store: &Store) -> StoreResult<(u64, u64, u64)> {
    let value = admission::header(store, &store.store_identity()?)?;
    Ok((value.bytes, value.roots, value.reservations))
}

pub(super) fn row(store: &Store, key: &[u8]) -> TestResult<Option<Vec<u8>>> {
    Ok(store
        .db
        .snapshot()
        .get(&store.journal, key)?
        .map(|raw| raw.to_vec()))
}

pub(super) fn header_bytes(store: &Store) -> TestResult<Vec<u8>> {
    row(store, admission::HEADER.as_bytes())?.ok_or_else(|| "admission header".into())
}

pub(super) fn physical_rows(store: &Store) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    store
        .db
        .snapshot()
        .prefix(&store.journal, b"j2\0")
        .take(MAX_PHYSICAL_ROWS + 1)
        .map(|guard| {
            let (key, value) = guard.into_inner()?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect()
}

pub(super) fn physical_bytes(store: &Store) -> TestResult<u64> {
    physical_rows(store)?
        .iter()
        .try_fold(0u64, |sum, (key, value)| {
            let length = key
                .len()
                .checked_add(value.len())
                .ok_or("physical row length")?;
            sum.checked_add(u64::try_from(length)?)
                .ok_or_else(|| "physical byte total".into())
        })
}

pub(super) fn write(store: &Store, key: &[u8], value: &[u8]) -> TestResult {
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, value);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

pub(super) fn remove(store: &Store, keys: &[Vec<u8>]) -> TestResult {
    let mut batch = store.db.batch();
    for key in keys {
        batch.remove(&store.journal, key);
    }
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

pub(super) fn load(store: &Store, operation: &str) -> StoreResult<Option<GraphReservation>> {
    load_graph(
        store,
        &store.store_identity()?,
        &RawGenerationOperationIdV2::new(operation)?,
    )
}
