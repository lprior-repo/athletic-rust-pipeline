//! Evidence-backed rankings divisions for the 2026 USA high-school scope.
//!
//! Source contract (captured `GetNavInfo` for USA level div `168416`): the
//! `seasons` map carries `"2026" -> 168416` for the outdoor list and
//! `"12026" -> 173005` for the indoor list. One division list serves both
//! genders behind the request `gender` parameter (`"m"`/`"f"`), so gender
//! selects a request division rather than a separate list id.

use serde::{Deserialize, Serialize};

/// Season year of the current scope.
pub const SEASON_YEAR: u64 = 2026;

/// Seasonal competition kind within the high-school year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeasonKind {
    Outdoor,
    Indoor,
}

/// Frozen scopes written before the division split were outdoor-only, and
/// `validate()` still rejects any list id the kind cannot serve.
impl Default for SeasonKind {
    fn default() -> Self {
        Self::Outdoor
    }
}

impl SeasonKind {
    /// Stable label used in revisions and operator output.
    pub fn label(self) -> &'static str {
        match self {
            Self::Outdoor => "outdoor",
            Self::Indoor => "indoor",
        }
    }

    /// Source season identifier this kind binds to. The captured `GetNavInfo`
    /// keys its `seasons` map by this value (`"2026"` outdoor, `"12026"`
    /// indoor) and the outdoor division page reports it as `SeasonID`; indoor
    /// seasons carry a leading `1` on the same year.
    pub fn season_id(self, season: u64) -> u64 {
        match self {
            Self::Indoor => 10_000u64.saturating_add(season),
            Self::Outdoor => season,
        }
    }

    /// `seasons` key for this kind, which is the source season id as text.
    pub fn seasons_key(self, season: u64) -> String {
        self.season_id(season).to_string()
    }
}

/// Evidence-backed division lists: (season kind, season, division list id).
const SEASON_LISTS: [(SeasonKind, u64, u64); 2] = [
    (SeasonKind::Outdoor, SEASON_YEAR, 168_416),
    (SeasonKind::Indoor, SEASON_YEAR, 173_005),
];

/// The source division-list id for a supported season.
pub fn season_list_id(kind: SeasonKind, season: u64) -> Option<u64> {
    SEASON_LISTS
        .iter()
        .find(|(candidate, year, _)| *candidate == kind && *year == season)
        .map(|(_, _, list_id)| *list_id)
}

/// Evidence-backed request gender codes.
pub fn normalize_gender(gender: &str) -> Option<&'static str> {
    match gender {
        "m" => Some("m"),
        "f" => Some("f"),
        _ => None,
    }
}

/// Durable revision for one seasonal division scope, or `None` for an
/// unsupported gender.
///
/// The pre-split `{SEASON_YEAR}-usa-boys-grade11-v1` revision is retired: a
/// scope written under it still deserializes (its absent `season_kind`
/// defaults to outdoor), but `RankingsScope::validate` rejects it with that
/// revision named. Snapshots carrying it are verified and exported by their
/// retained pre-change binary; this tree does not reinterpret them.
pub fn expected_revision(kind: SeasonKind, gender: &str) -> Option<String> {
    let label = match normalize_gender(gender)? {
        "m" => "boys",
        _ => "girls",
    };
    Some(format!(
        "{SEASON_YEAR}-usa-hs-grade11-{}-{label}-v2",
        kind.label()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn division_lists_match_the_captured_nav_contract() {
        assert_eq!(
            season_list_id(SeasonKind::Outdoor, SEASON_YEAR),
            Some(168_416)
        );
        assert_eq!(
            season_list_id(SeasonKind::Indoor, SEASON_YEAR),
            Some(173_005)
        );
        assert_eq!(SeasonKind::Outdoor.seasons_key(SEASON_YEAR), "2026");
        assert_eq!(SeasonKind::Indoor.seasons_key(SEASON_YEAR), "12026");
        assert_eq!(SeasonKind::Outdoor.season_id(SEASON_YEAR), 2_026);
        assert_eq!(SeasonKind::Indoor.season_id(SEASON_YEAR), 12_026);
        // The captured nav lists indoor seasons this way for every year it
        // covers, e.g. `"12025" -> 161984` and `"12024" -> 149057`.
        assert_eq!(SeasonKind::Indoor.season_id(2_025), 12_025);
        assert_eq!(SeasonKind::Indoor.season_id(2_024), 12_024);
    }

    #[test]
    fn unsupported_season_years_have_no_division_list() {
        assert_eq!(season_list_id(SeasonKind::Outdoor, 2025), None);
        assert_eq!(season_list_id(SeasonKind::Indoor, 2025), None);
    }

    #[test]
    fn revisions_distinguish_gender_and_season_kind() {
        assert_eq!(
            expected_revision(SeasonKind::Outdoor, "m").as_deref(),
            Some("2026-usa-hs-grade11-outdoor-boys-v2")
        );
        assert_eq!(
            expected_revision(SeasonKind::Indoor, "f").as_deref(),
            Some("2026-usa-hs-grade11-indoor-girls-v2")
        );
        assert_eq!(expected_revision(SeasonKind::Outdoor, "x"), None);
    }
}
