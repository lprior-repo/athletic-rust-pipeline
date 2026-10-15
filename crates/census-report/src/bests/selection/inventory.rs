use census_domain::model::{EventSemantics, PerformanceId};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionInventory {
    HistoricalContextGroups(HistoricalInventory),
    ResultGroups(ResultGroupInventory),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalInventory {
    policy: EventSemantics,
    eligible: usize,
    representatives: Vec<PerformanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultGroupInventory {
    eligible: usize,
    representatives: Vec<PerformanceId>,
}

impl SelectionInventory {
    pub(crate) fn from_groups(
        policy: EventSemantics,
        eligible: usize,
        mut representatives: Vec<PerformanceId>,
    ) -> Self {
        representatives.sort();
        representatives.dedup();
        match policy {
            EventSemantics::Legacy18
            | EventSemantics::FlatEvents19
            | EventSemantics::CombinedEvents20 => {
                Self::HistoricalContextGroups(HistoricalInventory {
                    policy,
                    eligible,
                    representatives,
                })
            }
            EventSemantics::ResultGroups21 => Self::ResultGroups(ResultGroupInventory {
                eligible,
                representatives,
            }),
        }
    }

    pub fn policy(&self) -> EventSemantics {
        match self {
            Self::HistoricalContextGroups(inventory) => inventory.policy,
            Self::ResultGroups(_) => EventSemantics::ResultGroups21,
        }
    }

    pub fn eligible_group_count(&self) -> usize {
        match self {
            Self::HistoricalContextGroups(inventory) => inventory.eligible,
            Self::ResultGroups(inventory) => inventory.eligible,
        }
    }

    pub fn tied_group_count(&self) -> usize {
        self.tied_representative_ids().len()
    }

    pub fn tied_representative_ids(&self) -> &[PerformanceId] {
        match self {
            Self::HistoricalContextGroups(inventory) => &inventory.representatives,
            Self::ResultGroups(inventory) => &inventory.representatives,
        }
    }

    pub(crate) fn serialized_field_count(&self) -> usize {
        match self {
            Self::HistoricalContextGroups(_) => 3,
            Self::ResultGroups(_) => 6,
        }
    }
}

impl Serialize for SelectionInventory {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut wire =
            serializer.serialize_struct("SelectionInventory", self.serialized_field_count())?;
        match self {
            Self::HistoricalContextGroups(_) => {
                wire.serialize_field("eligible_candidate_count", &self.eligible_group_count())?;
                wire.serialize_field("tied_performance_count", &self.tied_group_count())?;
                wire.serialize_field("tied_performance_ids", self.tied_representative_ids())?;
            }
            Self::ResultGroups(_) => {
                wire.serialize_field("eligible_result_group_count", &self.eligible_group_count())?;
                wire.serialize_field("tied_result_group_count", &self.tied_group_count())?;
                wire.serialize_field(
                    "tied_result_group_representative_ids",
                    self.tied_representative_ids(),
                )?;
                wire.serialize_field("result_group_basis", "canonical_performance_id")?;
                wire.serialize_field("physical_performance_count_status", "unknown")?;
                wire.serialize_field("selection_policy_version", &21_u32)?;
            }
        }
        wire.end()
    }
}

#[cfg(test)]
mod tests;
