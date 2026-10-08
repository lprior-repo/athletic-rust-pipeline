use census_domain::model::{
    AppliedAthleteIdentity, CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase,
    ReviewVerdictRecord, SourceAccessCondition, SourceObjectIdentity,
};

use crate::{Entity, StoreError, StoreResult, Table};

pub(super) fn fold(table: Table, prior: Option<&[u8]>, value: &[u8]) -> StoreResult<Vec<u8>> {
    match table {
        Table::SourceIdentities => fold_rows::<SourceObjectIdentity>(prior, value),
        Table::Conflicts => fold_rows::<RetainedConflict>(prior, value),
        Table::Coverage => fold_rows::<CoverageRow>(prior, value),
        Table::ReviewCases => fold_rows::<ReviewCase>(prior, value),
        Table::Snapshots => fold_rows::<CollectionSnapshot>(prior, value),
        Table::SourceAccess => fold_rows::<SourceAccessCondition>(prior, value),
        Table::IdentityVerdicts => fold_rows::<ReviewVerdictRecord>(prior, value),
        Table::AthleteIdentityDecisions => fold_rows::<AppliedAthleteIdentity>(prior, value),
        _ => Err(StoreError::Invariant {
            detail: format!("table {} is not stored as derived rows", table.file()),
        }),
    }
}

fn fold_rows<T: Entity>(prior: Option<&[u8]>, bytes: &[u8]) -> StoreResult<Vec<u8>> {
    let record: T = decode(bytes)?;
    let merged = match prior {
        Some(prior) => {
            let mut merged: T = decode(prior)?;
            merged.merge(record);
            merged
        }
        None => record,
    };
    serde_json::to_vec(&merged).map_err(|source| StoreError::Json {
        detail: "encoding migrated legacy derived row".into(),
        source,
    })
}

fn decode<T: Entity>(bytes: &[u8]) -> StoreResult<T> {
    serde_json::from_slice(bytes).map_err(|source| StoreError::Json {
        detail: "decoding migrated legacy derived row".into(),
        source,
    })
}
