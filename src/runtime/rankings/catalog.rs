use super::division::{SeasonKind, SEASON_YEAR};
use super::types::{is_excluded, NavEvent, RankedEvent, RankingsPlan};
use crate::domain::identity::EvidenceDigest;
use crate::runtime::protocol::RankingsCapture;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// A catalog of all observed event families from GetNavInfo,
/// classified into requested families with coverage tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCatalog {
    pub list_id: u64,
    pub level_div_id: u64,
    pub season_id: u64,
    /// All observed events from nav.events (excluding walk only).
    pub observed: Vec<NavEvent>,
    /// Mapped requested families with their observed variants.
    pub families: Vec<EventFamily>,
    /// Families from the manifest with no matching nav events.
    pub absent_families: Vec<AbsentFamily>,
}

/// One requested event family with its observed variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFamily {
    pub group: String,
    pub family: String,
    /// Observed variants (may be empty if family has no nav entries).
    pub variants: Vec<ObservedVariant>,
    /// Whether any variant was observed.
    pub has_observed: bool,
}

/// One observed event variant within a family.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedVariant {
    pub event: NavEvent,
    pub capture: RankingsCapture,
}

/// A family from the manifest with no matching events in nav.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsentFamily {
    pub group: String,
    pub family: String,
    pub short: String,
}

impl EventCatalog {
    /// Build a catalog from a GetNavInfo response and the requested event manifest.
    /// Walk is excluded. Shuttle hurdles (r+h) are preserved.
    ///
    /// The requested division is `expected_list_id`, which the nav must select
    /// through the kind-specific `seasons` key (outdoor `"2026"`, indoor
    /// `"12026"`). The nav itself is level-scoped, so its `divListId` is either
    /// that level or the selected list. The catalog's `list_id` is the selected
    /// list, which is what publication and verification compare against scope.
    pub fn from_nav(
        nav: &serde_json::Value,
        requested_families: &[RequestedFamily],
        season_kind: SeasonKind,
        expected_list_id: u64,
    ) -> Result<Self, CatalogError> {
        let (list_id, level_div_id) = Self::nav_scope(nav, season_kind, expected_list_id)?;
        let events_array = nav
            .get("events")
            .and_then(|value| value.as_array())
            .ok_or(CatalogError::MissingEvents)?;
        let observed = Self::observed_events(events_array)?;
        let (families, absent_families) = Self::family_coverage(&observed, requested_families);
        Ok(Self {
            list_id,
            level_div_id,
            season_id: season_kind.season_id(SEASON_YEAR),
            observed,
            families,
            absent_families,
        })
    }

    /// Resolve the nav's selected list and level division, rejecting any nav
    /// that does not select the expected list for the given season kind.
    fn nav_scope(
        nav: &serde_json::Value,
        season_kind: SeasonKind,
        expected_list_id: u64,
    ) -> Result<(u64, u64), CatalogError> {
        let nav_div_list_id = nav
            .get("divListId")
            .and_then(|value| value.as_u64())
            .ok_or(CatalogError::MissingDivListId)?;
        let level_div_id = nav
            .get("levelDivId")
            .and_then(|value| value.as_u64())
            .ok_or(CatalogError::MissingLevelDivId)?;
        let season_key = season_kind.seasons_key(SEASON_YEAR);
        let list_id = nav
            .get("seasons")
            .and_then(|value| value.as_object())
            .and_then(|seasons| seasons.get(&season_key))
            .and_then(|value| value.as_u64())
            .ok_or_else(|| CatalogError::MissingSeason {
                key: season_key.clone(),
            })?;
        if list_id != expected_list_id {
            return Err(CatalogError::SeasonListIdMismatch {
                key: season_key,
                expected: expected_list_id,
                actual: list_id,
            });
        }
        if nav_div_list_id != expected_list_id && nav_div_list_id != level_div_id {
            return Err(CatalogError::NavDivisionMismatch {
                nav_div_list_id,
                level_div_id,
                expected: expected_list_id,
            });
        }
        Ok((list_id, level_div_id))
    }

    /// Collect the nav's events in order, rejecting malformed entries and
    /// duplicate ids that disagree with the first observation.
    fn observed_events(events_array: &[serde_json::Value]) -> Result<Vec<NavEvent>, CatalogError> {
        let (_, observed) = events_array.iter().try_fold(
            (HashMap::<u64, usize>::new(), Vec::<NavEvent>::new()),
            |(mut seen_ids, mut observed), value| {
                let event = NavEvent::from_value(value).ok_or(CatalogError::MalformedNavEvent)?;
                if event.id == 0 || event.short.is_empty() || event.short.len() > 64 {
                    return Err(CatalogError::MalformedNavEvent);
                }
                if is_excluded(&event.short, event.r, event.h) {
                    return Ok((seen_ids, observed));
                }
                if let Some(index) = seen_ids.get(&event.id) {
                    let existing = observed
                        .get(*index)
                        .ok_or(CatalogError::MalformedNavEvent)?;
                    if existing != &event {
                        return Err(CatalogError::DuplicateEventId {
                            id: event.id,
                            first_short: existing.short.clone(),
                            second_short: event.short,
                        });
                    }
                    return Ok((seen_ids, observed));
                }
                seen_ids.insert(event.id, observed.len());
                observed.push(event);
                Ok((seen_ids, observed))
            },
        )?;
        Ok(observed)
    }

    /// Match every requested family against the observed events, recording the
    /// families that have no observed variant as absent.
    fn family_coverage(
        observed: &[NavEvent],
        requested_families: &[RequestedFamily],
    ) -> (Vec<EventFamily>, Vec<AbsentFamily>) {
        requested_families.iter().fold(
            (Vec::new(), Vec::new()),
            |(mut families, mut absent_families), requested| {
                let variants = observed
                    .iter()
                    .filter(|event| matches_requested(event, requested))
                    .cloned()
                    .map(|event| ObservedVariant {
                        event,
                        capture: RankingsCapture::Results,
                    })
                    .collect::<Vec<_>>();
                if variants.is_empty() {
                    absent_families.push(AbsentFamily {
                        group: requested.group.clone(),
                        family: requested.family.clone(),
                        short: requested.short.clone(),
                    });
                } else {
                    families.push(EventFamily {
                        group: requested.group.clone(),
                        family: requested.family.clone(),
                        variants,
                        has_observed: true,
                    });
                }
                (families, absent_families)
            },
        )
    }

    /// Convert catalog into a RankingsPlan with the given collection metadata.
    pub fn into_plan(
        self,
        collection: EvidenceDigest,
        grade: u8,
        gender: &str,
    ) -> Result<RankingsPlan, CatalogError> {
        let events = self
            .families
            .into_iter()
            .flat_map(|family| {
                let EventFamily {
                    group,
                    family,
                    variants,
                    has_observed: _,
                } = family;
                variants.into_iter().map(move |variant| {
                    let ObservedVariant { event, capture } = variant;
                    let NavEvent {
                        id: event_id,
                        short,
                        r: is_relay,
                        ..
                    } = event;
                    RankedEvent {
                        short,
                        family: family.clone(),
                        group: group.clone(),
                        event_id,
                        page: 1,
                        capture,
                        is_relay,
                    }
                })
            })
            .collect::<Vec<_>>();
        if events.is_empty() {
            return Err(CatalogError::NoMatchingEvents);
        }
        Ok(RankingsPlan {
            collection,
            list_id: self.list_id,
            gender: gender.to_owned(),
            grade,
            events,
        })
    }
}

fn matches_requested(event: &NavEvent, requested: &RequestedFamily) -> bool {
    match requested.short.as_str() {
        "" if requested.family == "sprint medley" => event.short.starts_with("sprintmed"),
        "" if requested.family == "distance medley" => event.short.starts_with("distmed"),
        "" if requested.family == "swedish relay" => event.short.starts_with("swedish"),
        "" if requested.family == "shuttle hurdle relay" => event.short.contains("shuttleh"),
        "" if requested.family == "pentathlon" => event.short.ends_with("pentathlon"),
        short => event.short == short,
    }
}

/// A requested event family from the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestedFamily {
    pub group: String,
    pub family: String,
    pub short: String,
}

/// Errors that can occur during catalog construction.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("missing divListId in nav response")]
    MissingDivListId,
    #[error("missing levelDivId in nav response")]
    MissingLevelDivId,
    #[error("missing season {key} in nav response")]
    MissingSeason { key: String },
    #[error("no matching events found for any requested family")]
    NoMatchingEvents,
    #[error("seasons['{key}'] must select the requested division list {expected}, got {actual}")]
    SeasonListIdMismatch {
        key: String,
        expected: u64,
        actual: u64,
    },
    #[error(
        "nav covers division {nav_div_list_id} outside level {level_div_id} and the requested list {expected}"
    )]
    NavDivisionMismatch {
        nav_div_list_id: u64,
        level_div_id: u64,
        expected: u64,
    },
    #[error("missing events array in nav response")]
    MissingEvents,
    #[error("malformed nav event")]
    MalformedNavEvent,
    #[error("duplicate event ID {id}: '{first_short}' vs '{second_short}'")]
    DuplicateEventId {
        id: u64,
        first_short: String,
        second_short: String,
    },
}
