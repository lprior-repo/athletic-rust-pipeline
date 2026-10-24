use super::{encoding, invalid, Entry, RecordedPayload, RetainedJournalBundle, MAX_PAYLOAD};
use census_domain::model::StoreIdentity;
use census_store::{JournalReadBudget, ReadBudget, StoreError, StoreResult, StoreSnapshot};
use serde::Serialize;
use serde_json::Value;
use std::io::Write;

mod count;
use count::CanonicalCount;

#[derive(Debug, Serialize)]
pub struct RetainedJournalPresentDemand {
    scope: &'static str,
    store_identity: StoreIdentity,
    snapshot_sequence: u64,
    phases: [PhaseDemand; 2],
    present_entries: u64,
    recorded_payload_bytes: u64,
    maximum_recorded_payload_bytes: u64,
    canonical_bundle_bytes: u64,
    canonical_digest: String,
    assumed_read_grant_bytes: usize,
}

#[derive(Debug, Serialize)]
struct PhaseDemand {
    phase: &'static str,
    logical_roots: u64,
    physical_rows: u64,
    physical_encoded_bytes: u64,
}

struct PresentDemand {
    canonical: CanonicalCount,
    entries: u64,
    payload_bytes: u64,
    maximum_payload_bytes: u64,
}

pub fn measure_retained_journal_present(
    view: &StoreSnapshot<'_>,
) -> StoreResult<RetainedJournalPresentDemand> {
    let store_identity = view.store_identity()?;
    let snapshot_sequence = view.sequence();
    let mut demand = PresentDemand::new(store_identity.clone(), snapshot_sequence)?;
    let capture = demand.phase(view, super::super::OWNED_CAPTURE_PHASE)?;
    let meet = demand.phase(view, super::super::OWNED_MEET_PHASE)?;
    write_fragment(&mut demand.canonical, b"]}")?;
    let (canonical_bundle_bytes, canonical_digest) = demand.canonical.finish();
    Ok(RetainedJournalPresentDemand {
        scope: "present_phase_superset",
        store_identity,
        snapshot_sequence,
        phases: [capture, meet],
        present_entries: demand.entries,
        recorded_payload_bytes: demand.payload_bytes,
        maximum_recorded_payload_bytes: demand.maximum_payload_bytes,
        canonical_bundle_bytes,
        canonical_digest,
        assumed_read_grant_bytes: MAX_PAYLOAD,
    })
}

impl PresentDemand {
    fn new(store_identity: StoreIdentity, snapshot_sequence: u64) -> StoreResult<Self> {
        let empty = RetainedJournalBundle {
            version: 1,
            store_identity,
            snapshot_sequence,
            entries: Vec::new(),
        };
        let header = serde_json::to_vec(&empty).map_err(encoding)?;
        let prefix = header
            .strip_suffix(b"]}")
            .ok_or_else(|| invalid("retained journal header has no entries terminator"))?;
        let mut canonical = CanonicalCount::new();
        write_fragment(&mut canonical, prefix)?;
        Ok(Self {
            canonical,
            entries: 0,
            payload_bytes: 0,
            maximum_payload_bytes: 0,
        })
    }

    fn phase(&mut self, view: &StoreSnapshot<'_>, phase: &'static str) -> StoreResult<PhaseDemand> {
        let read_budget = JournalReadBudget::new(MAX_PAYLOAD)?;
        let inventory = view.for_each_journal_root_demand_bounded(
            phase,
            ReadBudget {
                rows: 4_000_000,
                bytes: 16 * 1024 * 1024 * 1024,
                record_bytes: 1024 * 1024,
            },
            |name| {
                let (phase, key) = super::key(phase, name)?;
                let value = view
                    .journal_payload_bounded(&phase, &key, read_budget)?
                    .ok_or_else(|| invalid("enumerated journal root has no present payload"))?;
                self.observe(phase, key, value)
            },
        )?;
        Ok(PhaseDemand {
            phase,
            logical_roots: inventory.logical_roots(),
            physical_rows: inventory.fingerprint().physical_rows(),
            physical_encoded_bytes: inventory.fingerprint().encoded_bytes(),
        })
    }

    fn observe(&mut self, phase: String, key: String, value: Value) -> StoreResult<()> {
        let entry = Entry {
            phase,
            key,
            payload: RecordedPayload::Present {
                value,
                minimum_successful_read_budget: MAX_PAYLOAD,
            },
        };
        let mut payload = CanonicalCount::new();
        serde_json::to_writer(&mut payload, &entry.payload).map_err(encoding)?;
        if self.entries != 0 {
            write_fragment(&mut self.canonical, b",")?;
        }
        serde_json::to_writer(&mut self.canonical, &entry).map_err(encoding)?;
        self.entries = self
            .entries
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        self.payload_bytes = self
            .payload_bytes
            .checked_add(payload.bytes())
            .ok_or(StoreError::CounterOverflow)?;
        self.maximum_payload_bytes = self.maximum_payload_bytes.max(payload.bytes());
        Ok(())
    }
}

fn write_fragment(output: &mut CanonicalCount, bytes: &[u8]) -> StoreResult<()> {
    output
        .write_all(bytes)
        .map_err(|source| invalid(&format!("retained journal demand encoding: {source}")))
}

#[cfg(test)]
mod tests;
