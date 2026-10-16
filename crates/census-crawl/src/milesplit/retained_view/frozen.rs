use super::*;

pub struct FrozenRetainedJournal {
    identity: StoreIdentity,
    sequence: u64,
    entries: BTreeMap<(String, String), RecordedPayload>,
}

impl FrozenRetainedJournal {
    pub fn from_descriptor(
        expected_digest: &str,
        expected_identity: &str,
        expected_sequence: u64,
        bytes: &[u8],
    ) -> StoreResult<Self> {
        bound(bytes.len(), MAX_BYTES, "frozen retained journal bytes")?;
        if expected_sequence == 0 || crate::net::cache::content_digest(bytes) != expected_digest {
            return Err(invalid(
                "frozen retained journal differs from anchored descriptor",
            ));
        }
        let bundle: RetainedJournalBundle = serde_json::from_slice(bytes).map_err(encoding)?;
        if bundle.version != 1
            || bundle.store_identity.as_str() != expected_identity
            || bundle.snapshot_sequence != expected_sequence
            || codec::canonical_digest(&bundle)? != expected_digest
        {
            return Err(invalid(
                "frozen retained journal has foreign lineage or noncanonical encoding",
            ));
        }
        let entries = admitted_entries(bundle.entries)?;
        Ok(Self {
            identity: bundle.store_identity,
            sequence: bundle.snapshot_sequence,
            entries,
        })
    }
}

fn admitted_entries(
    entries: Vec<Entry>,
) -> StoreResult<BTreeMap<(String, String), RecordedPayload>> {
    bound(
        entries.len(),
        MAX_ENTRIES,
        "frozen retained journal entries",
    )?;
    let mut previous = None;
    entries
        .into_iter()
        .map(|entry| {
            let key = key(&entry.phase, &entry.key)?;
            if previous.as_ref().is_some_and(|previous| previous >= &key) {
                return Err(invalid(
                    "frozen retained journal keys are duplicate or unsorted",
                ));
            }
            previous = Some(key.clone());
            if let RecordedPayload::Present {
                value,
                minimum_successful_read_budget,
            } = &entry.payload
            {
                JournalReadBudget::new(*minimum_successful_read_budget)?;
                bound(
                    serde_json::to_vec(value).map_err(encoding)?.len(),
                    *minimum_successful_read_budget,
                    "frozen payload exceeds recorded successful read grant",
                )?;
            }
            bound(
                serde_json::to_vec(&entry.payload).map_err(encoding)?.len(),
                MAX_PAYLOAD,
                "frozen retained journal payload bytes",
            )?;
            Ok((key, entry.payload))
        })
        .collect()
}

impl super::sealed::Sealed for FrozenRetainedJournal {}
impl RetainedJournalView for FrozenRetainedJournal {
    fn store_identity(&self) -> StoreResult<StoreIdentity> {
        StoreIdentity::parse(self.identity.as_str())
            .map_err(|_| invalid("frozen retained journal identity changed"))
    }
    fn sequence(&self) -> u64 {
        self.sequence
    }
    fn journal_payload_bounded(
        &self,
        phase: &str,
        name: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>> {
        let key = key(phase, name)?;
        let payload = self.entries.get(&key).ok_or_else(|| {
            invalid("requested source object is outside frozen journal inventory")
        })?;
        match payload {
            RecordedPayload::Absent => Ok(None),
            RecordedPayload::Present {
                value,
                minimum_successful_read_budget,
            } => {
                if budget.max_encoded_bytes() < *minimum_successful_read_budget {
                    return Err(StoreError::JournalReadLimit {
                        phase: phase.into(),
                        key: name.into(),
                        what: "recorded successful read budget",
                        needed: *minimum_successful_read_budget,
                        max: budget.max_encoded_bytes(),
                    });
                }
                Ok(Some(value.clone()))
            }
        }
    }
}
