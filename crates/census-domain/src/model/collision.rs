use super::natural_key::NaturalKey;
use super::{Evidence, RetainedConflict, SourceIdentity};

pub const CANONICAL_ID_COLLISION_FAMILY: &str = "Canonical id collision";

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

pub(super) fn source_list(identities: &[&SourceIdentity]) -> String {
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
