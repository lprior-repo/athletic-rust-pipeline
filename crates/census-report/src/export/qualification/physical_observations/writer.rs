use super::*;
use std::path::PathBuf;

pub(in crate::export::qualification) struct Writer<'a> {
    dataset: &'a ExportDataset,
    table: Table,
    directory: PathBuf,
    buffer: Vec<u8>,
    chunks: Vec<ObjectRef>,
    closure: Closure,
}

impl<'a> Writer<'a> {
    pub(in crate::export::qualification) fn new(
        dataset: &'a ExportDataset,
        snapshot: &StoreSnapshot<'_>,
        table: Table,
        directory: &Path,
        budget: ReadBudget,
    ) -> ReportResult<Self> {
        dataset.ensure_input_binding()?;
        if snapshot.store_identity()?.as_str() != dataset.lineage.store_identity
            || snapshot.sequence() != dataset.lineage.snapshot_sequence
            || dataset.selected_profile_id().is_none()
            || dataset.selected_as_of().is_none()
        {
            return Err(invalid(
                "physical capture requires the selected input's exact pinned snapshot",
            )
            .into());
        }
        Ok(Self {
            dataset,
            table,
            directory: directory.to_path_buf(),
            buffer: Vec::new(),
            chunks: Vec::new(),
            closure: Closure::new(table, budget)?,
        })
    }

    pub(in crate::export::qualification) fn push(
        &mut self,
        key: &[u8],
        raw: &[u8],
    ) -> ReportResult<()> {
        self.closure.push(key, raw)?;
        let frame_bytes = key
            .len()
            .checked_add(raw.len())
            .and_then(|n| n.checked_add(8))
            .ok_or(StoreError::CounterOverflow)?;
        if self
            .buffer
            .len()
            .checked_add(frame_bytes)
            .ok_or(StoreError::CounterOverflow)?
            > CHUNK_BYTES
        {
            self.seal()?;
        }
        self.buffer
            .try_reserve(frame_bytes)
            .map_err(|_| invalid("physical row buffer allocation refused"))?;
        self.buffer.extend_from_slice(
            &u32::try_from(key.len())
                .map_err(|_| StoreError::CounterOverflow)?
                .to_be_bytes(),
        );
        self.buffer.extend_from_slice(
            &u32::try_from(raw.len())
                .map_err(|_| StoreError::CounterOverflow)?
                .to_be_bytes(),
        );
        self.buffer.extend_from_slice(key);
        self.buffer.extend_from_slice(raw);
        Ok(())
    }

    pub(in crate::export::qualification) fn finish(
        mut self,
        scanned: ReadCounts,
    ) -> ReportResult<PhysicalObservationDescriptor> {
        self.seal()?;
        let (counts, fingerprint) = self.closure.finish();
        if counts != scanned {
            return Err(invalid(
                "physical observation stream differs from its complete pinned walk",
            )
            .into());
        }
        let inventory = Inventory {
            version: 1,
            table: self.table,
            chunks: self.chunks,
            rows: counts.key_visits,
            bytes: counts.encoded_bytes,
            digest: fingerprint.clone(),
        };
        let reference = object::encode(&self.directory, &inventory)?;
        Ok(PhysicalObservationDescriptor {
            version: 1,
            store_identity: self.dataset.lineage.store_identity.clone(),
            snapshot_sequence: self.dataset.lineage.snapshot_sequence,
            input_generation: self.dataset.lineage.input_generation.clone(),
            input_digest: self.dataset.lineage.input_digest.clone(),
            source_digest: self.dataset.lineage.source_digest.clone(),
            profile_id: self
                .dataset
                .selected_profile_id()
                .ok_or_else(|| invalid("physical capture requires a selected profile"))?
                .clone(),
            as_of: self
                .dataset
                .selected_as_of()
                .ok_or_else(|| invalid("physical capture requires a selected cutoff"))?
                .clone(),
            table: self.table,
            inventory: reference,
            fingerprint,
            rows: counts.key_visits,
            bytes: counts.encoded_bytes,
        })
    }

    fn seal(&mut self) -> ReportResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        if self.chunks.len() >= MAX_CHUNKS {
            return Err(invalid(&format!(
                "physical row chunk inventory exceeds {MAX_CHUNKS} objects"
            ))
            .into());
        }
        let bytes = u64::try_from(self.buffer.len()).map_err(|_| StoreError::CounterOverflow)?;
        let digest = proof_objects::install(&self.directory, &self.buffer)?;
        self.chunks.push(ObjectRef { digest, bytes });
        self.buffer.clear();
        Ok(())
    }
}
