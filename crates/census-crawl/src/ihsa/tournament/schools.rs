use super::map::{Accumulator, Origin, ASSOCIATION};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalSchool, SchoolId, SourceIdentity, SourceNamespace,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::Table;
use std::collections::HashMap;

pub(super) struct Schools {
    by_association_id: HashMap<String, SchoolId>,
    index: SchoolIndex,
    resolved: HashMap<String, SchoolId>,
    minted: u64,
}

impl Schools {
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

    pub(super) fn minted(&self) -> u64 {
        self.minted
    }

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

    fn memo(&self, kind: &str, key: &str) -> Option<SchoolId> {
        self.resolved.get(&format!("{kind}:{key}")).cloned()
    }

    fn remember(&mut self, kind: &str, key: &str, found: SchoolId) -> SchoolId {
        self.resolved.insert(format!("{kind}:{key}"), found.clone());
        found
    }
}

pub(super) fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}
