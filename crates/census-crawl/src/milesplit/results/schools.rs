use super::{budget, ProviderSchools};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::CanonicalSchool;
use census_store::{StoreError, Table};

pub(super) fn read(ctx: &AdapterContext<'_>) -> CrawlResult<ProviderSchools> {
    let mut schools = ProviderSchools::default();
    ctx.store
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            schools.absorb(&school);
            if schools.entries() > budget::SCHOOL_BINDINGS {
                return Err(StoreError::Invariant {
                    detail: "result school-binding resource limit; projection remains unfinished"
                        .into(),
                });
            }
            Ok(())
        })?;
    Ok(schools)
}
