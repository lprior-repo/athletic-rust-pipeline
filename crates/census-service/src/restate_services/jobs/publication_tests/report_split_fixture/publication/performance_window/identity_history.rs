use super::{Fixture, TestResult};
use census_domain::public_identity_history::PublicAthleteId;
use std::collections::BTreeMap;

type IdentityMap = BTreeMap<String, PublicAthleteId>;

mod source {
    include!("../../../../../../../../census-report/src/export/dataset/selected/tests/publication/performance_window/identity_history/source.rs");
}
mod correction {
    include!("../../../../../../../../census-report/src/export/dataset/selected/tests/publication/performance_window/identity_history/correction.rs");
}

pub(super) fn merge(
    fixture: &Fixture,
    alias: census_domain::model::CanonicalAthlete,
) -> TestResult<(String, PublicAthleteId, BTreeMap<String, String>)> {
    let (case_id, canonical, merged_ids, _alias_proof) = source::merge(fixture, alias)?;
    let identities = merged_ids
        .into_iter()
        .map(|(candidate, public)| (candidate, public.as_str().to_owned()))
        .collect();
    Ok((case_id, canonical, identities))
}

pub(super) fn split(
    fixture: &Fixture,
    case_id: &str,
    canonical: &PublicAthleteId,
) -> TestResult<IdentityMap> {
    correction::split(fixture, case_id, canonical)
}
