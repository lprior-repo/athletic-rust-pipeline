//! The finding one canonical-id collision produces, and how its two sides print.
//!
//! [`id_collision`] names the id, both sides' material and both sides' sources, so an operator acts on
//! the two subjects rather than on a count of them. It is the second half of the natural-key check in
//! [`super::natural_key`]: that module answers whether two rows are one subject, this one writes down
//! what it means when they are not.

use super::natural_key::NaturalKey;
use super::{Evidence, RetainedConflict, SourceIdentity};

/// The family label a canonical-id collision carries, as it is stored in the `family` column.
///
/// The families the workbook renders live with [`RetainedConflict`] in `model::records`; this one
/// lives next to the detector that raises it, and stays a constant so a reader that matches the
/// family by name cannot spell it differently from the writer that stores it.
pub const CANONICAL_ID_COLLISION_FAMILY: &str = "Canonical id collision";

/// Mint the retained conflict for one canonical id that two different natural keys produced.
///
/// The kept row survives untouched and this finding says which subject it is, what the id was minted
/// from this time, and what it was minted from the other time — the two subjects an operator has to
/// tell apart, not a count of collisions.
pub fn id_collision<K: NaturalKey, D: NaturalKey>(
    id: &str,
    kept: &K,
    dropped: &D,
) -> RetainedConflict {
    let subject = kept.natural_key();
    let detail = format!(
        "canonical id {id} was minted from two different natural keys: kept {subject} [{}]; dropped {} [{}]",
        kept.sources(),
        dropped.natural_key(),
        dropped.sources(),
    );
    RetainedConflict::new(CANONICAL_ID_COLLISION_FAMILY, id, subject, detail)
}

/// One side's provider identities as a conflict line spells them: `namespace:id`, with the URL when
/// the row carries one.
pub(super) fn source_list(identities: &[SourceIdentity]) -> String {
    if identities.is_empty() {
        return "-".to_string();
    }
    identities
        .iter()
        .map(|identity| match identity.url.as_deref() {
            Some(url) => format!("{}:{} ({url})", identity.namespace, identity.id),
            None => format!("{}:{}", identity.namespace, identity.id),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The evidence sources a row carries, deduplicated, for the rows that keep no identity list: an
/// event's and a performance's provider ids live in the store's identity table (§31), so their own
/// columns are what a collision can name.
pub(super) fn evidence_list(evidence: &[Evidence]) -> String {
    let mut sources: Vec<String> = Vec::new();
    for item in evidence {
        let rendered = match item.source.url.as_deref() {
            Some(url) => format!("{} ({url})", item.source.id),
            None => item.source.id.clone(),
        };
        if !sources.contains(&rendered) {
            sources.push(rendered);
        }
    }
    if sources.is_empty() {
        return "-".to_string();
    }
    sources.join(", ")
}
