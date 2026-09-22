//! School resolution: the IHSA's own school ids first, published names second.
//!
//! The association-id map is built from the identities the `ihsa` schools adapter already minted
//! (`AssociationSchool { association: "ihsa" }`), so a run that follows a schools pull resolves every
//! finisher onto the canonical school instead of minting a second one. A row the map does not know
//! falls back to the name index the result-file adapters use, and a row both miss mints a school under
//! this adapter's own association id — never a school with no name, because school identity is
//! (jurisdiction, normalized name).

use super::map::{Accumulator, Origin, ASSOCIATION};
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, CrawlResult};
use crate::store::Table;
use census_domain::model::{
    normalize_name, CanonicalSchool, SchoolId, SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

/// The schools a run resolves against.
pub(super) struct Schools {
    by_association_id: HashMap<String, SchoolId>,
    index: SchoolIndex,
    resolved: HashMap<String, SchoolId>,
    minted: u64,
}

impl Schools {
    /// Load the store's schools into the two lookup tables.
    pub(super) fn load(ctx: &AdapterContext<'_>) -> CrawlResult<Self> {
        let schools: Vec<CanonicalSchool> = ctx.store.scan(Table::Schools)?;
        let mut by_association_id = HashMap::new();
        for school in &schools {
            for identity in &school.source_identities {
                if let SourceNamespace::AssociationSchool { association } = &identity.namespace {
                    if association == ASSOCIATION {
                        by_association_id
                            .entry(identity.id.clone())
                            .or_insert_with(|| school.id.clone());
                    }
                }
            }
        }
        Ok(Self {
            by_association_id,
            index: SchoolIndex::from_schools(&schools),
            resolved: HashMap::new(),
            minted: 0,
        })
    }

    /// How many schools this run minted because neither table knew the row's school.
    pub(super) fn minted(&self) -> u64 {
        self.minted
    }

    /// Resolve the school one published row names.
    ///
    /// `None` when the row publishes neither an id nor a name: a row that carries neither cannot be
    /// filed, and minting a school from nothing would invent one.
    pub(super) fn resolve(
        &mut self,
        ihsa_id: Option<&str>,
        name: Option<&str>,
        url: &str,
        origin: Origin<'_>,
        accumulated: &mut Accumulator,
    ) -> Option<SchoolId> {
        let published = trimmed(ihsa_id);
        if let Some(key) = published {
            if let Some(found) = self.memo("id", key) {
                return Some(found);
            }
            if let Some(found) = self.by_association_id.get(key).cloned() {
                return Some(self.remember("id", key, found));
            }
        }
        let label = trimmed(name)?;
        let normalized = normalize_name(label);
        if let Some(found) = self.memo("name", &normalized) {
            return Some(found);
        }
        if let Some((found, _)) = self.index.resolve(UsJurisdiction::Illinois, label) {
            return Some(self.remember("name", &normalized, found));
        }
        Some(self.mint(label, &normalized, published, url, origin, accumulated))
    }

    /// Mint a school the association-id map and the name index both missed.
    ///
    /// `IHSA` publishes `ihsaSchoolId` on every finisher row, so the id it does know is kept as the
    /// school's association identity: the next run resolves it from the map instead of the name.
    fn mint(
        &mut self,
        name: &str,
        normalized: &str,
        published: Option<&str>,
        url: &str,
        origin: Origin<'_>,
        accumulated: &mut Accumulator,
    ) -> SchoolId {
        let (mut school, id) = CanonicalSchool::new(UsJurisdiction::Illinois, name, normalized);
        school.association = Some(ASSOCIATION.to_string());
        if let Some(key) = published {
            school.source_identities.push(SourceIdentity::new(
                SourceNamespace::AssociationSchool {
                    association: ASSOCIATION.to_string(),
                },
                key,
            ));
            self.resolved.insert(format!("id:{key}"), id.clone());
        }
        school.evidence.push(origin.evidence(url));
        self.minted = self.minted.saturating_add(1);
        accumulated
            .schools
            .entry(id.as_str().to_string())
            .or_insert(school);
        id
    }

    /// A resolution this run has already made.
    fn memo(&self, kind: &str, key: &str) -> Option<SchoolId> {
        self.resolved.get(&format!("{kind}:{key}")).cloned()
    }

    /// Remember a resolution for the rest of the run.
    fn remember(&mut self, kind: &str, key: &str, found: SchoolId) -> SchoolId {
        self.resolved.insert(format!("{kind}:{key}"), found.clone());
        found
    }
}

/// A published field, trimmed, or `None` when the row leaves it empty.
pub(super) fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}
