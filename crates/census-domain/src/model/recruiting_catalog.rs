use super::{
    serialized_digest, write_serialized, CanonicalEvent, CanonicalJsonError, EventKind,
    SourceEventLabel, Sport,
};
use serde::Serialize;

mod aliases;
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

fn coherent_components(label: &str, kind: &EventKind) -> Result<bool, CatalogError> {
    aliases::tokens(label).try_fold(true, |coherent, word| {
        let component = EventKind::from_source_label(word);
        let matches = match component {
            EventKind::Unmapped { .. } if distance_quantity(word) => coherent_distance(word, kind)?,
            EventKind::Unmapped { .. } => true,
            _ => component == *kind || distance_component(kind, &component),
        };
        Ok(coherent && matches)
    })
}

fn excluded_coherence(
    event: &CanonicalEvent,
    reason: EventExclusionReason,
) -> Result<EventDisposition, CatalogError> {
    if matches!(
        event.resolved_specification(),
        Err(super::SpecificationError::ConflictingSpecification)
    ) {
        return Ok(EventDisposition::Unresolved(
            EventResolutionReason::ContradictoryDeclarations,
        ));
    }
    let mut kind = None;
    for label in &event.source_labels {
        let mut preceding: Option<&str> = None;
        for word in aliases::tokens(&label.label) {
            let unit = aliases::member(word, policy::UNITS)
                && preceding.is_some_and(|previous| {
                    previous
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || byte == b'.')
                        || previous.eq_ignore_ascii_case("two")
                });
            preceding = Some(word);
            if unit {
                continue;
            }
            let component = aliases::excluded_component(word);
            if let Some(component) = component {
                if kind.as_ref().is_some_and(|old| old != &component) {
                    return Ok(EventDisposition::Unresolved(
                        EventResolutionReason::ContradictoryDeclarations,
                    ));
                }
                kind = Some(component);
            }
        }
        let mut buffer = String::new();
        for (left, right) in
            aliases::tokens(&label.label).zip(aliases::tokens(&label.label).skip(1))
        {
            if !(left
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
                || left.eq_ignore_ascii_case("two"))
                || !aliases::member(right, policy::UNITS)
            {
                continue;
            }
            let bytes = left
                .len()
                .checked_add(right.len())
                .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
            buffer.clear();
            buffer
                .try_reserve(bytes)
                .map_err(|_| CatalogError::ResourceExhausted)?;
            buffer.push_str(left);
            buffer.push_str(right);
            let component = EventKind::from_source_label(&buffer);
            if kind.as_ref().is_some_and(|old| old != &component) {
                return Ok(EventDisposition::Unresolved(
                    EventResolutionReason::ContradictoryDeclarations,
                ));
            }
            kind = Some(component);
        }
    }
    Ok(EventDisposition::Excluded(reason))
}

fn coherent_pairs(label: &str, kind: &EventKind) -> Result<bool, CatalogError> {
    let mut buffer = String::new();
    for (left, right) in aliases::tokens(label).zip(aliases::tokens(label).skip(1)) {
        let bytes = left
            .len()
            .checked_add(right.len())
            .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
        buffer.clear();
        buffer
            .try_reserve(bytes)
            .map_err(|_| CatalogError::ResourceExhausted)?;
        buffer.push_str(left);
        buffer.push_str(right);
        let component = EventKind::from_source_label(&buffer);
        match component {
            EventKind::Unmapped { .. } if distance_quantity(&buffer) => {
                if !coherent_distance(&buffer, kind)? {
                    return Ok(false);
                }
            }
            EventKind::Unmapped { .. } => {}
            _ if component == *kind || distance_component(kind, &component) => {}
            _ => return Ok(false),
        }
    }
    Ok(true)
}

fn distance_quantity(word: &str) -> bool {
    aliases::quantity(word)
        && word
            .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
            .and_then(|end| word.get(end..))
            .is_some_and(|unit| {
                aliases::member(
                    unit,
                    &[
                        "m", "meter", "meters", "metre", "metres", "k", "km", "mile", "miles",
                    ],
                )
            })
}

fn coherent_distance(word: &str, kind: &EventKind) -> Result<bool, CatalogError> {
    let suffix = match kind {
        EventKind::CrossCountry | EventKind::SprintMedley | EventKind::DistanceMedley => {
            return Ok(true)
        }
        EventKind::Track2000mSteeplechase | EventKind::Track3000mSteeplechase => "steeplechase",
        _ if kind.is_hurdles() => "hurdles",
        _ => return Ok(false),
    };
    let bytes = word
        .len()
        .checked_add(suffix.len())
        .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
    let mut qualified = String::new();
    qualified
        .try_reserve(bytes)
        .map_err(|_| CatalogError::ResourceExhausted)?;
    qualified.push_str(word);
    qualified.push_str(suffix);
    Ok(EventKind::from_source_label(&qualified) == *kind)
}

fn distance_component(kind: &EventKind, component: &EventKind) -> bool {
    match kind {
        EventKind::Track55mHurdles => component == &EventKind::Track55m,
        EventKind::Track60mHurdles => component == &EventKind::Track60m,
        EventKind::Track100mHurdles => component == &EventKind::Track100m,
        EventKind::Track400mHurdles => component == &EventKind::Track400m,
        EventKind::Track3000mSteeplechase => component == &EventKind::Track3000m,
        EventKind::CrossCountry | EventKind::SprintMedley | EventKind::DistanceMedley => {
            matches!(
                component,
                EventKind::Track55m
                    | EventKind::Track60m
                    | EventKind::Track100m
                    | EventKind::Track200m
                    | EventKind::Track400m
                    | EventKind::Track800m
                    | EventKind::Track1000m
                    | EventKind::Track1500m
                    | EventKind::Track1600m
                    | EventKind::Track1Mile
                    | EventKind::Track3000m
                    | EventKind::Track3200m
                    | EventKind::Track5000m
            )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
