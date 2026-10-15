use std::fmt;

use crate::model::StoreIdentity;
use serde::de::{DeserializeSeed, Error, SeqAccess, Visitor};
use serde::Deserializer;

use super::{
    replay_candidate_history, HistoryProjection, HistoryRecord, RegistryHead, ReplayLimits,
};

mod atoms;
mod containers;
mod namespace;
mod objects;
mod requests;

use containers::{Meter, NodeSeed, RecordNode};

pub struct CandidateHistorySeed<'a> {
    installation: &'a StoreIdentity,
    expected_head: &'a RegistryHead,
    limits: ReplayLimits,
}

pub struct DecodedCandidateHistory {
    records: Vec<HistoryRecord>,
    projection: HistoryProjection,
}

impl<'a> CandidateHistorySeed<'a> {
    pub fn new(
        installation: &'a StoreIdentity,
        expected_head: &'a RegistryHead,
        limits: ReplayLimits,
    ) -> Self {
        Self {
            installation,
            expected_head,
            limits,
        }
    }
}

impl DecodedCandidateHistory {
    pub fn records(&self) -> &[HistoryRecord] {
        &self.records
    }

    pub fn projection(&self) -> &HistoryProjection {
        &self.projection
    }

    pub fn into_parts(self) -> (Vec<HistoryRecord>, HistoryProjection) {
        (self.records, self.projection)
    }
}

impl<'de> DeserializeSeed<'de> for CandidateHistorySeed<'_> {
    type Value = DecodedCandidateHistory;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for CandidateHistorySeed<'_> {
    type Value = DecodedCandidateHistory;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded version-one candidate identity history")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut meter = Meter::new(self.limits);
        let mut records = Vec::new();
        containers::admit_hint(sequence.size_hint(), self.limits.records)?;
        for _ in 0..self.limits.records {
            let Some(record) =
                sequence.next_element_seed(NodeSeed::<RecordNode>::new(&mut meter))?
            else {
                return self.finish(records);
            };
            records.try_reserve(1).map_err(A::Error::custom)?;
            records.push(record);
        }
        containers::require_end(&mut sequence)?;
        self.finish(records)
    }
}

impl CandidateHistorySeed<'_> {
    fn finish<E: Error>(self, records: Vec<HistoryRecord>) -> Result<DecodedCandidateHistory, E> {
        let projection =
            replay_candidate_history(self.installation, self.expected_head, &records, self.limits)
                .map_err(E::custom)?;
        Ok(DecodedCandidateHistory {
            records,
            projection,
        })
    }
}

#[cfg(test)]
mod tests;
