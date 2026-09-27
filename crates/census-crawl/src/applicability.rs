mod table;

use crate::registry::{bulk_first, SourceDescriptor};
use census_domain::UsJurisdiction;

use table::TABLE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applicability {
    pub slug: &'static str,
    pub jurisdictions: &'static [UsJurisdiction],
    pub evidence: &'static str,
    pub refusal: &'static str,
}

pub fn table() -> &'static [Applicability] {
    &TABLE
}

pub fn applicable_sources(jurisdiction: UsJurisdiction) -> Vec<&'static SourceDescriptor> {
    let slugs: Vec<&str> = TABLE
        .iter()
        .filter(|row| row.jurisdictions.contains(&jurisdiction))
        .map(|row| row.slug)
        .collect();
    bulk_first(&slugs)
}

#[cfg(test)]
#[path = "applicability/tests.rs"]
mod tests;
