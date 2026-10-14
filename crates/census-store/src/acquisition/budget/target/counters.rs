use super::{
    allocation::Allocations,
    counter::{product, Counter},
    limits,
};
use crate::StoreResult;

#[derive(Debug, Clone, Copy)]
pub(in crate::acquisition) struct ReadCounters {
    pub(in crate::acquisition) roots: Counter,
    pub(in crate::acquisition) encoded: Counter,
    pub(in crate::acquisition) receipts: Counter,
    pub(in crate::acquisition) admission: Counter,
    pub(in crate::acquisition) admission_probes: Counter,
    pub(in crate::acquisition) allocations: Allocations,
}
impl ReadCounters {
    pub(in crate::acquisition) fn new(count: u64) -> StoreResult<Self> {
        Ok(Self {
            roots: Counter::new(product(count, limits::ROOTS)?),
            encoded: Counter::new(product(count, limits::ENCODED)?),
            receipts: Counter::new(product(count, limits::RECEIPTS)?),
            admission: Counter::new(product(count, limits::ADMISSION)?),
            admission_probes: Counter::new(u64::MAX),
            allocations: Allocations::new(count)?,
        })
    }
    pub(in crate::acquisition) const fn empty() -> Self {
        Self {
            roots: Counter::new(0),
            encoded: Counter::new(0),
            receipts: Counter::new(0),
            admission: Counter::new(0),
            admission_probes: Counter::new(u64::MAX),
            allocations: Allocations::empty(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ReadDebit {
    Row(u64),
    Encoded(u64),
    Receipt,
    Admission(u64),
    AdmissionProbe,
    Allocate(super::allocation::AllocationKind, u64),
    ReleaseOwned(u64),
}
impl ReadDebit {
    pub(super) fn permits_closed(self) -> bool {
        matches!(self, Self::Allocate(_, _) | Self::ReleaseOwned(_))
    }
}
impl ReadCounters {
    pub(super) fn charged(mut self, debit: ReadDebit) -> StoreResult<Self> {
        match debit {
            ReadDebit::Row(bytes) => {
                self.roots = self.roots.charged(1, "roots")?;
                self.encoded = self.encoded.charged(bytes, "encoded bytes")?;
            }
            ReadDebit::Encoded(bytes) => {
                self.encoded = self.encoded.charged(bytes, "encoded bytes")?
            }
            ReadDebit::Receipt => self.receipts = self.receipts.charged(1, "receipt probes")?,
            ReadDebit::Admission(bytes) => {
                self.admission = self.admission.charged(bytes, "admission encoded bytes")?
            }
            ReadDebit::AdmissionProbe => {
                self.admission_probes = self.admission_probes.charged(1, "admission probes")?
            }
            ReadDebit::Allocate(kind, bytes) => {
                self.allocations = self.allocations.reserved(kind, bytes)?
            }
            ReadDebit::ReleaseOwned(bytes) => {
                self.allocations = self.allocations.released(bytes)?
            }
        }
        Ok(self)
    }
}
