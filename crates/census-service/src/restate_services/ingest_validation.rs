use serde::de::DeserializeOwned;
use serde_json::Value;

use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow,
    RetainedConflict, ReviewCase, ReviewVerdictRecord, SourceAccessCondition, SourceMeetRef,
    SourceObjectIdentity, SourceObservation,
};
use census_store::Table;

use super::JobError;

pub(super) fn validate_rows(table: Table, rows: &[Value]) -> Result<(), JobError> {
    for (index, row) in rows.iter().enumerate() {
        validate_row(table, index, row)?;
    }
    Ok(())
}

fn validate_row(table: Table, index: usize, row: &Value) -> Result<(), JobError> {
    match table {
        Table::Schools => decode_row::<CanonicalSchool>(table, index, row),
        Table::Teams => decode_row::<CanonicalTeam>(table, index, row),
        Table::Coaches => decode_row::<CanonicalCoach>(table, index, row),
        Table::Athletes => decode_row::<CanonicalAthlete>(table, index, row),
        Table::Meets => decode_row::<CanonicalMeet>(table, index, row),
        Table::Events => decode_row::<CanonicalEvent>(table, index, row),
        Table::Performances => decode_row::<CanonicalPerformance>(table, index, row),
        Table::SourceIdentities => decode_row::<SourceObjectIdentity>(table, index, row),
        Table::Conflicts => decode_row::<RetainedConflict>(table, index, row),
        Table::ReviewCases => decode_row::<ReviewCase>(table, index, row),
        Table::Coverage => decode_row::<CoverageRow>(table, index, row),
        Table::Snapshots => decode_row::<CollectionSnapshot>(table, index, row),
        Table::SourceAccess => decode_row::<SourceAccessCondition>(table, index, row),
        Table::IdentityVerdicts => decode_row::<ReviewVerdictRecord>(table, index, row),
        Table::SourceMeets => decode_row::<SourceMeetRef>(table, index, row),
        Table::SourceObservations => decode_row::<SourceObservation>(table, index, row),
        Table::AthleteIdentityDecisions => decode_row::<AppliedAthleteIdentity>(table, index, row),
    }
}

fn decode_row<T>(table: Table, index: usize, row: &Value) -> Result<(), JobError>
where
    T: DeserializeOwned,
{
    T::deserialize(row)
        .map(|_| ())
        .map_err(|source| JobError::Terminal {
            message: format!(
                "row {index} for table {} failed validation: {source}",
                table.file()
            ),
        })
}
