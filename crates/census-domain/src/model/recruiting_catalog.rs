use super::EventSemantics;
use super::{
    serialized_digest, write_serialized, CanonicalEvent, CanonicalJsonError, EventKind,
    SourceEventLabel, Sport,
};
use serde::Serialize;

mod aliases;
mod coherence;
mod context;
use coherence::{coherent_components, coherent_pairs, distance_quantity, excluded_coherence};
mod disposition;
mod legacy_policy;
mod policy;
mod selected;

pub use disposition::{
    CatalogError, EventDisposition, EventExclusionReason, EventResolutionReason,
};

pub const CATALOG_V1_MAX_SOURCE_LABELS: usize = 256;
pub const CATALOG_V1_MAX_LABEL_BYTES: usize = 4096;
pub const CATALOG_V1_MAX_AGGREGATE_LABEL_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecruitingCatalogV1 {
    version: CatalogVersion,
    semantics: EventSemantics,
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
        Self::with_policy(EventSemantics::CURRENT)
    }

    pub const fn with_policy(semantics: EventSemantics) -> Self {
        Self {
            version: CatalogVersion::V1,
            semantics,
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
            policy_revision: match self.semantics {
                EventSemantics::Legacy18 => None,
                EventSemantics::FlatEvents19 => Some(2),
                EventSemantics::CombinedEvents20 | EventSemantics::ResultGroups21 => Some(3),
            },
            grammar: match self.semantics {
                EventSemantics::Legacy18 => "ascii_delimited_tokens_typed_context_xc_anchors_v3",
                EventSemantics::FlatEvents19
                | EventSemantics::CombinedEvents20
                | EventSemantics::ResultGroups21 => {
                    "ascii_delimited_tokens_typed_context_xc_anchors_v4"
                }
            },
            coherence: "all_owned_assertions_canonical_kind_sport_category_v1",
            combined_completion: self
                .semantics
                .requires_completed_combined()
                .then_some("versioned_lineup_scoring_complete_components_exact_points_v1"),
            max_source_labels: CATALOG_V1_MAX_SOURCE_LABELS,
            max_label_bytes: CATALOG_V1_MAX_LABEL_BYTES,
            max_aggregate_label_bytes: CATALOG_V1_MAX_AGGREGATE_LABEL_BYTES,
            included: policy::included(self.semantics),
            exclusions: policy::EXCLUSIONS,
            pairs: policy::PAIRS,
            terrain: policy::TERRAIN,
            ambiguous: policy::AMBIGUOUS,
            missing: match self.semantics {
                EventSemantics::Legacy18 => legacy_policy::MISSING,
                EventSemantics::FlatEvents19
                | EventSemantics::CombinedEvents20
                | EventSemantics::ResultGroups21 => policy::MISSING,
            },
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
                Ok::<_, CatalogError>(declarations.add(aliases::classify(label, self.semantics)?))
            },
        )?;
        match declarations.finish() {
            EventDisposition::Included(kind) => {
                disposition::qualify(event, sport, kind, self.semantics)
            }
            EventDisposition::Excluded(reason) => excluded_coherence(event, reason, self.semantics),
            other => Ok(other),
        }
    }
}

#[derive(Serialize)]
struct CatalogIdentity<'a> {
    version: CatalogVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy_revision: Option<u32>,
    grammar: &'a str,
    coherence: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    combined_completion: Option<&'a str>,
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
