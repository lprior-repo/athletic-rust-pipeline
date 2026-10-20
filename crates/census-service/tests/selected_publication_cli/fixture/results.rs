use super::{Store, Table, TestResult, UsJurisdiction};
use census_domain::model::{
    CanonicalPerformance, CanonicalSchool, SourceIdentity, SourceNamespace,
};

pub(super) fn append(store: &Store) -> TestResult<CanonicalPerformance> {
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Synthetic selected school",
        "selected school",
        None,
    )
    .0;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "38332",
    ));
    store.append(Table::Schools, &school)?;
    let performance = super::golden::append(store)?;
    super::golden_journal::replay(store)?;
    Ok(performance)
}
