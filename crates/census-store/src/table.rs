use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const MAX_ROWS_PER_TABLE: u64 = 20_000_000;

pub const MAX_ID_BYTES: usize = 512;

pub const MAX_JOURNAL_KEY_BYTES: usize = 4 * 1024;

pub const MAX_JOURNAL_VALUE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    ObservationLog,
    DerivedGeneration,
    DerivedMap,
}

impl StorageMode {
    pub const fn appended_observations(self, rows: u64) -> u64 {
        match self {
            StorageMode::ObservationLog => rows,
            StorageMode::DerivedGeneration | StorageMode::DerivedMap => 0,
        }
    }

    pub const fn generation_partitioned(self) -> bool {
        matches!(self, StorageMode::DerivedGeneration)
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
    RelayResults,
    CoachLanes,
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
            Table::RelayResults => "relay_results",
            Table::CoachLanes => "coach_lanes",
        }
    }

    pub const ALL: [Table; 19] = [
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
        Table::RelayResults,
        Table::CoachLanes,
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
            | Table::SourceObservations
            | Table::RelayResults => StorageMode::ObservationLog,
            Table::SourceIdentities
            | Table::Conflicts
            | Table::Coverage
            | Table::ReviewCases
            | Table::Snapshots => StorageMode::DerivedGeneration,
            Table::SourceAccess
            | Table::IdentityVerdicts
            | Table::AthleteIdentityDecisions
            | Table::CoachLanes => StorageMode::DerivedMap,
        }
    }

    pub const fn generation_partitioned(self) -> bool {
        self.storage_mode().generation_partitioned()
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
    use census_domain::model::{
        CoachLaneAcceptance, CoachLaneEvidence, CoachLaneObligation, CoachLaneRefusal,
        CoachLaneResolution, CoachLaneStage, PendingCoachLane, SchoolYear, SourceRef,
    };
    use census_domain::UsJurisdiction;

    const OBSERVED_ON: &str = "2026-10-11";
    const DIGEST: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0";

    fn pending(
        jurisdiction: UsJurisdiction,
        arm: &str,
    ) -> Result<CoachLaneObligation, Box<dyn std::error::Error>> {
        Ok(CoachLaneObligation::pending(PendingCoachLane {
            jurisdiction,
            season: SchoolYear::new(2026).ok_or("school year 2026")?,
            revision: 1,
            arm: arm.to_string(),
            arm_basis: "association directory lists coach roles".to_string(),
            observed_on: OBSERVED_ON.to_string(),
        })?)
    }

    #[test]
    fn a_table_serializes_as_its_wire_name() -> Result<(), Box<dyn std::error::Error>> {
        for table in Table::ALL {
            let encoded = serde_json::to_string(&table)?;
            check!(eq; encoded, format!("\"{}\"", table.file()));
            let decoded: Table = serde_json::from_str(&encoded)?;
            check!(eq; decoded, table);
        }
        Ok(())
    }

    #[test]
    fn coach_lane_rows_round_trip_through_the_coach_lanes_table(
    ) -> Result<(), Box<dyn std::error::Error>> {
        check!(eq; Table::CoachLanes.storage_mode(), StorageMode::DerivedMap);
        check!(eq; Table::from_wire("coach_lanes"), Some(Table::CoachLanes));
        check!(Table::ALL.contains(&Table::CoachLanes));
        let armed = pending(UsJurisdiction::Ohio, "arbiter_orgs")?.arm(
            CoachLaneEvidence::new(41, 63, DIGEST, OBSERVED_ON)?,
            CoachLaneResolution::unresolved(OBSERVED_ON),
        )?;
        let recorded = pending(UsJurisdiction::Michigan, "mhsaa_directory")?.record(
            CoachLaneStage::Blocked,
            Some(CoachLaneRefusal::new(
                SourceRef::id("mhsaa_directory"),
                "no coach roles are published for this association",
                OBSERVED_ON,
            )?),
            CoachLaneResolution::unresolved(OBSERVED_ON),
        )?;
        let refused = recorded.accept_refusal(CoachLaneAcceptance::new(
            "census-ops",
            "jpx4.accepted_refusal",
            OBSERVED_ON,
        )?)?;
        check!(eq; armed.id(), "coach_lane:OH:2026:1");
        check!(eq; refused.id(), "coach_lane:MI:2026:1");
        let dir = tempfile::tempdir()?;
        let store = crate::Store::open(dir.path())?;
        store.replace(
            Table::CoachLanes,
            &pending(UsJurisdiction::Ohio, "arbiter_orgs")?,
        )?;
        store.replace(Table::CoachLanes, &armed)?;
        store.replace(Table::CoachLanes, &refused)?;
        let rows: Vec<CoachLaneObligation> = store.scan(Table::CoachLanes)?;
        check!(eq; rows.len(), 2);
        check!(
            rows.contains(&armed),
            "the armed row is what the table retains for OH"
        );
        check!(rows.contains(&refused));
        check!(eq; store.walk_table(Table::CoachLanes)?.rows, 2);
        drop(store);
        let reopened = crate::Store::open(dir.path())?;
        let after: Vec<CoachLaneObligation> = reopened.scan(Table::CoachLanes)?;
        check!(eq; after.len(), 2);
        check!(after.contains(&armed));
        check!(after.contains(&refused));
        Ok(())
    }
}
