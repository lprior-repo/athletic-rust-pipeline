use std::collections::BTreeSet;

use crate::UsJurisdiction;

use super::super::key::IdentifiedKey;
use super::super::label::SourceLabel;
use super::{AttestedRecord, IndexEntry, MatchForm};

pub(super) fn corroborated(
    entry: &IndexEntry,
    city: Option<&MatchForm>,
    attested: &[AttestedRecord],
) -> bool {
    let IdentifiedKey::StateRecord { id, .. } = &entry.key else {
        return true;
    };
    if attested
        .iter()
        .any(|record| &record.id == id && record.label == entry.source)
    {
        return true;
    }
    match (city, entry.city.as_ref()) {
        (Some(school), Some(published)) => school == published,
        _ => false,
    }
}

pub(super) fn record_label(
    sources: &BTreeSet<SourceLabel>,
    state: UsJurisdiction,
) -> Option<SourceLabel> {
    let mut chosen: Option<SourceLabel> = None;
    for label in sources {
        let eligible = match label {
            SourceLabel::StateEducationAgency { state: label_state }
            | SourceLabel::AthleticAssociation { state: label_state } => *label_state == state,
            SourceLabel::PrivateAssociation { .. } => true,
            _ => false,
        };
        if !eligible {
            continue;
        }
        if chosen
            .as_ref()
            .is_none_or(|current| label.rank() < current.rank())
        {
            chosen = Some(label.clone());
        }
    }
    chosen
}
