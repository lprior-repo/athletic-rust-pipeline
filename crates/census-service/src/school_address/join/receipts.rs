use census_domain::model::{serialized_digest, CanonicalSchool, ReviewCase, SchoolId};
use census_store::{Application, Store, StoreError, StoreResult, StoreRevision, Table};
use serde::Serialize;

mod budget;
mod effects;
mod scope;
mod witness;

pub(super) use scope::ApplicationScope;

const APPLICATION_VERSION: &str = "school-address-join-application/v1";
const WITNESS_PHASE: &str = "school-address-join-application-v1";

pub(super) fn apply_target<'s>(
    store: &'s Store,
    scope: &ApplicationScope,
    target: &SchoolId,
    schools: &[CanonicalSchool],
    cases: &[ReviewCase],
    expected: StoreRevision<'s>,
) -> StoreResult<StoreRevision<'s>> {
    if schools.is_empty() && cases.is_empty() {
        return Ok(expected);
    }
    let effects = effects::TargetEffects::new(target, schools, cases)?;
    let witness = witness::JoinApplicationWitness::new(scope, target, &effects, expected)?;
    effects.admit_witness(&witness)?;
    let digest = digest(&witness)?;
    let operation = format!("school-address-join:application:v1:{digest}");
    effects.admit_application(&witness, &operation, &digest)?;
    let mut batch = store.write_batch();
    batch.record_many(Table::Schools, &effects.schools)?;
    batch.record_many(Table::ReviewCases, &effects.cases)?;
    batch.journal_done(WITNESS_PHASE, &operation, &witness)?;
    let (application, revision) = batch
        .commit_once_at_revision(&operation, &digest, expected)?
        .into_parts();
    match application {
        Application::Written(_) => Ok(revision),
        Application::Repeated(receipt) => Err(StoreError::Refused {
            detail: format!(
                "school-address application {} already exists; recompute from a new coherent \
                 snapshot before reporting current effects",
                receipt.operation
            ),
        }),
    }
}

fn digest<T: Serialize + ?Sized>(value: &T) -> StoreResult<String> {
    serialized_digest(value).map_err(|error| StoreError::Invariant {
        detail: format!("school-address application is not canonical JSON: {error}"),
    })
}

fn reserve<T>(values: &mut Vec<T>, additional: usize) -> StoreResult<()> {
    values
        .try_reserve_exact(additional)
        .map_err(|error| StoreError::Refused {
            detail: format!("allocating bounded school-address application: {error}"),
        })
}

fn copy_string(value: &str) -> StoreResult<String> {
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|error| StoreError::Refused {
            detail: format!("allocating school-address provenance text: {error}"),
        })?;
    owned.push_str(value);
    Ok(owned)
}

fn copy_optional(value: &Option<String>) -> StoreResult<Option<String>> {
    value.as_deref().map(copy_string).transpose()
}
