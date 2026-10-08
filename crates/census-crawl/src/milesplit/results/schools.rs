use super::{budget, ProviderSchools};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::CanonicalSchool;
use census_store::{StoreError, Table};

pub(super) fn read(ctx: &AdapterContext<'_>) -> CrawlResult<ProviderSchools> {
    let mut schools = Vec::new();
    let mut bytes = 0usize;
    let mut identities = 0usize;
    ctx.store
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            let footprint = budget::Footprint::of(&school).map_err(invariant)?;
            bytes = bytes
                .checked_add(footprint.bytes)
                .ok_or(StoreError::CounterOverflow)?;
            identities = identities
                .checked_add(school.source_identities.len())
                .ok_or(StoreError::CounterOverflow)?;
            if schools.len() >= budget::SCHOOL_ROWS
                || bytes > budget::SCHOOL_BYTES
                || identities > budget::SCHOOL_ROWS
            {
                return Err(StoreError::Invariant {
                    detail: "result school-binding resource limit; projection remains unfinished"
                        .into(),
                });
            }
            schools
                .try_reserve(1)
                .map_err(|error| invariant(budget::reserve(error)))?;
            schools.push(school);
            Ok(())
        })?;
    Ok(ProviderSchools::from_schools(&schools))
}

fn invariant(error: crate::CrawlError) -> StoreError {
    StoreError::Invariant {
        detail: error.to_string(),
    }
}
