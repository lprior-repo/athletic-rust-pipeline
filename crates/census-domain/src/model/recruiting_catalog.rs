use super::{
    serialized_digest, write_serialized, CanonicalEvent, CanonicalJsonError, EventKind,
    SourceEventLabel, Sport,
};
use serde::Serialize;

mod aliases;
mod coherence;
use coherence::{coherent_components, coherent_pairs, distance_quantity, excluded_coherence};
mod disposition;
mod policy;

pub use disposition::{
    CatalogError, EventDisposition, EventExclusionReason, EventResolutionReason,
};

pub const CATALOG_V1_MAX_SOURCE_LABELS: usize = 256;
pub const CATALOG_V1_MAX_LABEL_BYTES: usize = 4096;
pub const CATALOG_V1_MAX_AGGREGATE_LABEL_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecruitingCatalogV1 {
    version: CatalogVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CatalogVersion {
    V1,
}

impl Default for RecruitingCatalogV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl RecruitingCatalogV1 {
    pub const fn new() -> Self {
        Self {
            version: CatalogVersion::V1,
        }
    }

    pub fn digest(&self) -> Result<String, CanonicalJsonError> {
        serialized_digest(&self.identity())
    }

    pub fn write_policy(&self, writer: &mut impl std::io::Write) -> Result<(), CanonicalJsonError> {
        write_serialized(writer, &self.identity())
    }

    fn identity(&self) -> CatalogIdentity<'static> {
        CatalogIdentity {
            version: self.version,
            grammar: "ascii_delimited_tokens_typed_context_xc_anchors_v3",
            coherence: "all_owned_assertions_canonical_kind_sport_category_v1",
            max_source_labels: CATALOG_V1_MAX_SOURCE_LABELS,
            max_label_bytes: CATALOG_V1_MAX_LABEL_BYTES,
            max_aggregate_label_bytes: CATALOG_V1_MAX_AGGREGATE_LABEL_BYTES,
            included: policy::INCLUDED,
            exclusions: policy::EXCLUSIONS,
            pairs: policy::PAIRS,
            terrain: policy::TERRAIN,
            ambiguous: policy::AMBIGUOUS,
            missing: policy::MISSING,
            qualifiers: policy::QUALIFIERS,
            units: policy::UNITS,
            separators: policy::SEPARATORS,
        }
    }

    pub fn classify(
        &self,
        event: &CanonicalEvent,
        sport: Sport,
    ) -> Result<EventDisposition, CatalogError> {
        disposition::admit(event)?;
        if !event.retained_conflicts.is_empty() {
            return Ok(EventDisposition::Unresolved(
                EventResolutionReason::ContradictoryDeclarations,
            ));
        }
        if event.source_labels.is_empty()
            || event.source_labels.iter().any(|label| {
                label.source.id.trim().is_empty() || !event.trusted_specification_label(label)
            })
        {
            return Ok(EventDisposition::Unresolved(
                EventResolutionReason::MissingSourceOwnership,
            ));
        }
        let declarations = event.source_labels.iter().try_fold(
            disposition::Declarations::new(),
            |declarations, label| {
                Ok::<_, CatalogError>(declarations.add(aliases::classify(label)?))
            },
        )?;
        match declarations.finish() {
            EventDisposition::Included(kind) => disposition::qualify(event, sport, kind),
            EventDisposition::Excluded(reason) => excluded_coherence(event, reason),
            other => Ok(other),
        }
    }
}

#[derive(Serialize)]
struct CatalogIdentity<'a> {
    version: CatalogVersion,
    grammar: &'a str,
    coherence: &'a str,
    max_source_labels: usize,
    max_label_bytes: usize,
    max_aggregate_label_bytes: usize,
    included: &'a [EventKind],
    exclusions: &'a [(&'a str, EventExclusionReason)],
    pairs: &'a [(&'a str, &'a str, EventExclusionReason)],
    terrain: &'a [&'a str],
    ambiguous: &'a [&'a str],
    missing: &'a [&'a str],
    qualifiers: &'a [&'a str],
    units: &'a [&'a str],
    separators: &'a str,
}

#[cfg(test)]
mod tests;
