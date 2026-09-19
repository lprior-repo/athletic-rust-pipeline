use crate::domain::error::DomainError;
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use crate::runtime::protocol::RankingsCapture;
use crate::runtime::source::request::RankingsAction;
use crate::runtime::rankings::catalog::RequestedFamily;
use serde::{Deserialize, Serialize};

/// A complete rankings collection plan covering all requested
/// event families for a single division scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingsPlan {
    pub collection: EvidenceDigest,
    pub list_id: u64,
    pub gender: String,
    pub grade: u8,
    pub events: Vec<RankedEvent>,
}

/// Rankings scope: immutable collection parameters derived from scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingsScope {
    pub revision: String,
    pub list_id: u64,
    pub season: u64,
    pub gender: String,
    pub projection_grade: u8,
    pub country: String,
    pub level: u64,
    pub max_pages_per_event: u32,
    pub requested_families: Vec<RequestedFamily>,
}
impl RankingsScope {
    /// Construct the canonical 2026 USA boys Grade 11 scope with all 46
    /// requested event families. The caller supplies only the per-event
    /// page cap; everything else is fixed by the specification.
    pub fn requested(max_pages_per_event: u32) -> anyhow::Result<Self> {
        if max_pages_per_event == 0 || max_pages_per_event > 10_000 {
            anyhow::bail!("max_pages_per_event must be 1..=10000");
        }
        let families = Self::default_families();
        Ok(Self {
            revision: "2026-usa-boys-grade11-v1".to_owned(),
            list_id: 168416,
            season: 2026,
            gender: "m".to_owned(),
            projection_grade: 11,
            country: "USA".to_owned(),
            level: 4,
            max_pages_per_event,
            requested_families: families,
        })
    }

    /// Only the canonical source scope may enter the durable collection.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.list_id != 168416 || self.revision != "2026-usa-boys-grade11-v1" {
            anyhow::bail!("unsupported ranking list or revision");
        }
        if self.country != "USA" {
            anyhow::bail!("unsupported country: {}", self.country);
        }
        if self.level != 4 {
            anyhow::bail!("unsupported level: {}", self.level);
        }
        if self.season != 2026 {
            anyhow::bail!("unsupported season: {}", self.season);
        }
        if self.gender != "m" {
            anyhow::bail!("unsupported gender: {}", self.gender);
        }
        if self.projection_grade != 11 {
            anyhow::bail!("unsupported projection_grade: {}", self.projection_grade);
        }
        if self.max_pages_per_event == 0 || self.max_pages_per_event > 10_000 {
            anyhow::bail!("max_pages_per_event must be 1..=10000");
        }
        let manifest_matches = self.requested_families.len() == REQUESTED_FAMILY_MANIFEST.len()
            && self.requested_families.iter().zip(REQUESTED_FAMILY_MANIFEST).all(
                |(actual, (group, family, short))| {
                    actual.group == group && actual.family == family && actual.short == short
                },
            );
        if !manifest_matches {
            anyhow::bail!("rankings scope must include the complete requested family manifest");
        }
        Ok(())
    }

    /// The 46 requested families for 2026 USA high school boys Grade 11.
    /// The catalog expands these dynamically into ~95 observed variants.
    fn default_families() -> Vec<RequestedFamily> {
        REQUESTED_FAMILY_MANIFEST
            .iter()
            .map(|(group, family, short)| RequestedFamily {
                group: (*group).to_owned(),
                family: (*family).to_owned(),
                short: (*short).to_owned(),
            })
            .collect()
    }
}

const REQUESTED_FAMILY_MANIFEST: [(&str, &str, &str); 46] = [
    ("sprint", "55m", "55m"),
    ("sprint", "60m", "60m"),
    ("sprint", "100m", "100m"),
    ("sprint", "200m", "200m"),
    ("sprint", "300m", "300m"),
    ("sprint", "400m", "400m"),
    ("middle", "500m", "500m"),
    ("middle", "600m", "600m"),
    ("middle", "800m", "800m"),
    ("middle", "1000m", "1000m"),
    ("distance", "1500m", "1500m"),
    ("distance", "1600m", "1600m"),
    ("distance", "mile", "1mile"),
    ("distance", "2 mile", "2miles"),
    ("distance", "3000m", "3000m"),
    ("distance", "3200m", "3200m"),
    ("steeplechase", "2000mSC", "2ksteeple"),
    ("steeplechase", "3000mSC", "3ksteeple"),
    ("hurdles", "55mH", "55mh"),
    ("hurdles", "60mH", "60mh"),
    ("hurdles", "100mH", "100mh"),
    ("hurdles", "110mH", "110mh"),
    ("hurdles", "300mH", "300mh"),
    ("hurdles", "400mH", "400mh"),
    ("relay", "4x100", "4x100m"),
    ("relay", "4x200", "4x200m"),
    ("relay", "4x400", "4x400m"),
    ("relay", "4x800", "4x800m"),
    ("relay", "4x1600", "4x1600m"),
    ("relay", "4×mile", "4xmile"),
    ("relay", "sprint medley", ""),
    ("relay", "distance medley", ""),
    ("relay", "swedish relay", ""),
    ("jump", "high jump", "hj"),
    ("relay", "shuttle hurdle relay", ""),
    ("jump", "long jump", "lj"),
    ("jump", "triple jump", "tj"),
    ("jump", "pole vault", "pv"),
    ("throw", "shot put", "shot"),
    ("throw", "discus", "discus"),
    ("throw", "javelin", "javelin"),
    ("throw", "hammer", "hammer"),
    ("throw", "weight throw", "weight"),
    ("combined", "pentathlon", ""),
    ("combined", "heptathlon", "heptathlon"),
    ("combined", "decathlon", "decathlon"),
];

/// One event family within a RankingsPlan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedEvent {
    pub short: String,
    pub family: String,
    pub group: String,
    pub event_id: u64,
    pub page: u32,
    pub capture: RankingsCapture,
    pub is_relay: bool,
}

impl RankingsPlan {
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn for_event(&self, short: &str) -> Option<&RankedEvent> {
        self.events.iter().find(|e| e.short == short)
    }

    pub fn build_action(&self, event: &RankedEvent, page: u32) -> RankingsAction {
        RankingsAction {
            collection: self.collection.clone(),
            list_id: self.list_id,
            gender: self.gender.clone(),
            grade: if event.is_relay {
                None
            } else {
                Some(self.grade)
            },
            event_short: event.short.clone(),
            page,
            capture: event.capture.clone(),
        }
    }
}

/// Relay roster member from the application UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMember {
    #[serde(rename = "SortID")]
    pub sort_id: u64,
    #[serde(rename = "IDAthlete")]
    pub athlete_id: u64,
    #[serde(rename = "AthleteName")]
    pub name: String,
    pub handle: String,
    #[serde(rename = "PhotoUrl", default)]
    pub photo_url: Option<String>,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
}

/// Relay team from the application UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayTeam {
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    #[serde(rename = "RelayTeamID")]
    pub team_id: u64,
    pub members: Vec<RelayMember>,
}

/// A relay ranking row with the join keys for roster matching.
/// All relay rows have GradeID=99 (system placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayRow {
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    #[serde(rename = "AthleteID")]
    pub athlete_id: u64,
    #[serde(rename = "TeamID")]
    pub team_id: u64,
    #[serde(rename = "AthleteName")]
    pub athlete_name: String,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
}

/// A verified relay member: member data + row provenance.
/// Only emitted when roster.RelayTeamID == row.AthleteID and member.GradeID=11.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedRelayMember {
    #[serde(rename = "IDAthlete")]
    pub athlete_id: AthleteId,
    #[serde(rename = "AthleteName")]
    pub name: CanonicalName,
    #[serde(rename = "Handle")]
    pub handle: String,
    #[serde(rename = "PhotoUrl", default)]
    pub photo_url: Option<String>,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    pub roster_relay_team_id: u64,
    pub row_athlete_id: u64,
    /// JSON pointer to source row: groupedRankings/{group}/{row}
    pub row_locator: Option<String>,
    /// JSON pointer to source member: relayTeams/{IDResult}/Members/{index}
    pub member_locator: Option<usize>,
}

/// Relay roster indexed by IDResult string key.
pub type RelayRoster = std::collections::HashMap<String, RelayTeam>;

/// Validated event metadata from GetNavInfo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavEvent {
    pub id: u64,
    pub name: String,
    pub t: String,
    pub m: String,
    pub r: bool,
    pub h: bool,
    pub short: String,
    pub w: bool,
    pub fmt: Option<String>,
    pub so: u64,
}

/// Excluded event patterns: walk/race-walk only.
pub fn is_excluded(short: &str, _r: bool, _h: bool) -> bool {
    short
        .as_bytes()
        .windows(4)
        .any(|window| window.eq_ignore_ascii_case(b"walk"))
}

impl NavEvent {
    /// Parse a NavEvent from a borrowed JSON value without cloning the value.
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: value.get("id")?.as_u64()?,
            name: value.get("name")?.as_str()?.to_owned(),
            t: value.get("t")?.as_str()?.to_owned(),
            m: value.get("m")?.as_str()?.to_owned(),
            r: value.get("r")?.as_bool()?,
            h: value.get("h")?.as_bool()?,
            short: value.get("short")?.as_str()?.to_owned(),
            w: value.get("w")?.as_bool()?,
            fmt: value
                .get("fmt")
                .map(|item| item.as_str().map(str::to_owned))
                .transpose()?,
            so: value.get("so")?.as_u64()?,
        })
    }
}

/// An individual GradeID=11 athlete candidate with row provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualCandidate {
    #[serde(rename = "AthleteID")]
    pub athlete_id: AthleteId,
    #[serde(rename = "AthleteName")]
    pub name: CanonicalName,
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
    #[serde(rename = "TeamID")]
    pub team_id: Option<u64>,
    #[serde(rename = "TeamName")]
    pub team_name: Option<String>,
    #[serde(rename = "State")]
    pub state: Option<String>,
    #[serde(rename = "Country")]
    pub country: Option<String>,
    /// JSON pointer to the source row: groupedRankings/{group}/{row}
    /// Relative index in the corresponding observation candidate vector.
    pub record_index: u64,
    pub source_locator: Option<String>,
}
/// Returns source-located eligible candidate records plus counts.
/// Bounded observation from a parsed source page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageObservation {
    pub row_count: u64,
    pub id_results: Vec<u64>,
    pub grade_11_candidates: u64,
    pub total_relay_rows: u64,
    pub rows_with_roster: u64,
    pub rows_missing_roster: u64,
    pub total_relay_teams: u64,
    pub division_id: Option<u64>,
    pub season_id: Option<u64>,
    pub request_gender: Option<String>,
    pub request_page: Option<u32>,
    pub event_short: Option<String>,
    pub settings_page_depth: u64,
    /// Individual GradeID=11 athlete candidates with provenance.
    pub grade_11_candidates_list: Vec<IndividualCandidate>,
    pub verified_relay_members: Vec<VerifiedRelayMember>,
    /// Source-located ranking rows with provenance.
    pub source_rows: Vec<RankingRowObservation>,
    /// Lower bound of rows observed across all pages.
    pub min_count: u64,
    /// Count of individual identities that could not be resolved.
    pub unresolved_individual_identities: u64,
    /// Count of relay member identities that could not be resolved.
    pub unresolved_member_identities: u64,
}

impl PageObservation {
    pub fn is_valid(&self, div: u64, season: u64, page: u32) -> bool {
        self.division_id == Some(div)
            && self.season_id == Some(season)
            && self.request_page == Some(page)
    }
}

/// Typed expected page context for parser validation.
/// Native collection always Some(11) individual/None relay.
/// Historical diagnostic individual all-grade capture may explicitly expect None.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedPageContext<'a> {
    pub division_id: u64,
    pub season_id: u64,
    pub gender: &'a str,
    pub event_short: &'a str,
    pub event_id: Option<u64>,
    pub is_relay: bool,
    pub requested_grade: Option<u8>,
    pub page: u32,
}

/// One ranking row observation with provenance.
/// Row positions from actual rowNum, never per-page enumeration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRowObservation {
    pub result_id: u64,
    pub row_number: u64,
    pub roster_present: Option<bool>,
}

