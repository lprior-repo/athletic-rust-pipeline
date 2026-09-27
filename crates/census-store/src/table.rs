use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const MAX_ROWS_PER_TABLE: u64 = 20_000_000;

pub const MAX_ID_BYTES: usize = 512;

pub const MAX_JOURNAL_KEY_BYTES: usize = 4 * 1024;

pub const MAX_JOURNAL_VALUE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    ObservationLog,
    DerivedSnapshot,
    DerivedMap,
}

impl StorageMode {
    pub const fn appended_observations(self, rows: u64) -> u64 {
        match self {
            StorageMode::ObservationLog => rows,
            StorageMode::DerivedSnapshot | StorageMode::DerivedMap => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    Schools,
    Teams,
    Coaches,
    Athletes,
    Meets,
    Events,
    Performances,
    SourceIdentities,
    Conflicts,
    ReviewCases,
    Coverage,
    Snapshots,
    SourceAccess,
    IdentityVerdicts,
    SourceMeets,
    SourceObservations,
    AthleteIdentityDecisions,
}

impl Table {
    pub fn file(self) -> &'static str {
        match self {
            Table::Schools => "schools",
            Table::Teams => "teams",
            Table::Coaches => "coaches",
            Table::Athletes => "athletes",
            Table::Meets => "meets",
            Table::Events => "events",
            Table::Performances => "performances",
            Table::SourceIdentities => "source_identities",
            Table::Conflicts => "conflicts",
            Table::ReviewCases => "review_cases",
            Table::Coverage => "coverage",
            Table::Snapshots => "snapshots",
            Table::SourceAccess => "source_access",
            Table::IdentityVerdicts => "identity_verdicts",
            Table::SourceMeets => "source_meets",
            Table::SourceObservations => "source_observations",
            Table::AthleteIdentityDecisions => "athlete_identity_decisions",
        }
    }

    pub const ALL: [Table; 17] = [
        Table::Schools,
        Table::Teams,
        Table::Coaches,
        Table::Athletes,
        Table::Meets,
        Table::Events,
        Table::Performances,
        Table::SourceIdentities,
        Table::Conflicts,
        Table::ReviewCases,
        Table::Coverage,
        Table::Snapshots,
        Table::SourceAccess,
        Table::IdentityVerdicts,
        Table::SourceMeets,
        Table::SourceObservations,
        Table::AthleteIdentityDecisions,
    ];

    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.file() == name)
    }

    pub const fn storage_mode(self) -> StorageMode {
        match self {
            Table::Schools
            | Table::Teams
            | Table::Coaches
            | Table::Athletes
            | Table::Meets
            | Table::Events
            | Table::Performances
            | Table::SourceMeets
            | Table::SourceObservations => StorageMode::ObservationLog,
            Table::SourceIdentities | Table::Conflicts | Table::Coverage => {
                StorageMode::DerivedSnapshot
            }
            Table::ReviewCases
            | Table::Snapshots
            | Table::SourceAccess
            | Table::IdentityVerdicts
            | Table::AthleteIdentityDecisions => StorageMode::DerivedMap,
        }
    }
}

pub trait Entity: Serialize + DeserializeOwned + Clone {
    fn entity_id(&self) -> &str;
    fn merge(&mut self, other: Self);

    fn publish(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_serializes_as_its_wire_name() {
        for table in Table::ALL {
            let encoded = serde_json::to_string(&table).expect("a table encodes");
            assert_eq!(encoded, format!("\"{}\"", table.file()));
            let decoded: Table = serde_json::from_str(&encoded).expect("a table decodes");
            assert_eq!(decoded, table);
        }
    }
}
