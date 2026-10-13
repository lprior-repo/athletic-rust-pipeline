use census_domain::model::SchoolId;
use census_store::{StoreResult, StoreRevision, Table};
use serde::Serialize;

use super::effects::TargetEffects;
use super::scope::ScopeProvenance;
use super::{digest, ApplicationScope, APPLICATION_VERSION};

#[derive(Serialize)]
pub(super) struct JoinApplicationWitness<'a> {
    application_version: &'static str,
    effects: Vec<EffectBinding<'a>>,
    mutation: MutationKind,
    provenance: &'a ScopeProvenance,
    scope_digest: &'a str,
    target: &'a SchoolId,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum MutationKind {
    Schools,
    ReviewCases,
    SchoolsAndReviewCases,
}

#[derive(Serialize)]
struct EffectBinding<'a> {
    payload_digest: String,
    placement: Placement,
    table: &'static str,
    target: &'a SchoolId,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum Placement {
    Unpartitioned,
    DerivedGeneration(u64),
}

impl<'a> JoinApplicationWitness<'a> {
    pub(super) fn new(
        scope: &'a ApplicationScope,
        target: &'a SchoolId,
        payloads: &TargetEffects<'_>,
        expected: StoreRevision<'_>,
    ) -> StoreResult<Self> {
        let mut effects = Vec::new();
        super::reserve(&mut effects, 2)?;
        if !payloads.schools.is_empty() {
            effects.push(EffectBinding {
                table: Table::Schools.file(),
                target,
                payload_digest: digest(&payloads.schools)?,
                placement: Placement::Unpartitioned,
            });
        }
        if !payloads.cases.is_empty() {
            effects.push(EffectBinding {
                table: Table::ReviewCases.file(),
                target,
                payload_digest: digest(&payloads.cases)?,
                placement: Placement::DerivedGeneration(expected.derived_generation()),
            });
        }
        let mutation = match (payloads.schools.is_empty(), payloads.cases.is_empty()) {
            (false, true) => MutationKind::Schools,
            (true, false) => MutationKind::ReviewCases,
            (false, false) => MutationKind::SchoolsAndReviewCases,
            (true, true) => {
                return Err(census_store::StoreError::Invariant {
                    detail: "an empty school-address application has no witness".to_string(),
                });
            }
        };
        Ok(Self {
            application_version: APPLICATION_VERSION,
            scope_digest: &scope.digest,
            provenance: &scope.provenance,
            target,
            mutation,
            effects,
        })
    }
}
