use census_domain::UsJurisdiction;
use sha2::{Digest, Sha256};

use super::{config::Profile, corpus, TestResult};
use crate::restate_services::results_arms::selection::Selected;

pub(super) fn selected(
    selected: &Selected,
    profile: &Profile,
    jurisdiction: UsJurisdiction,
    owner: usize,
    digest: &mut Sha256,
) -> TestResult {
    let owners = UsJurisdiction::CENSUS_SCOPE.len();
    let start = u64::try_from(owner)?;
    let indices = || (start..profile.rows).step_by(owners);
    let mut expected = indices().filter(|index| index % 100 == 0);
    selected.meets.iter().try_for_each(|actual| {
        let index = expected.next().ok_or("extra admitted row")?;
        let row = corpus::row(profile, jurisdiction, index);
        check!(eq; actual, &row);
        frame(digest, &serde_json::to_vec(actual)?)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    check!(expected.next().is_none());
    let mut expected = indices().filter(|index| index % 100 == 1);
    selected.pending.iter().try_for_each(|actual| {
        let index = expected.next().ok_or("extra pending row")?;
        let row = corpus::row(profile, jurisdiction, index);
        check!(eq; actual, &row.results_url);
        frame(digest, actual.as_bytes())?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    check!(expected.next().is_none());
    Ok(())
}

pub(super) fn partition_rows(profile: &Profile, owner: usize) -> TestResult<u64> {
    let count = (u64::try_from(owner)?..profile.rows)
        .step_by(UsJurisdiction::CENSUS_SCOPE.len())
        .count();
    Ok(u64::try_from(count)?)
}

fn frame(digest: &mut Sha256, bytes: &[u8]) -> TestResult {
    digest.update(u64::try_from(bytes.len())?.to_le_bytes());
    digest.update(bytes);
    Ok(())
}
