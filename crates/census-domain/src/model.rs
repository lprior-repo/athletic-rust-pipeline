//! Canonical model for the independent Midwest HS TF/XC recruiting graph.
//!
//! Identity rules enforced here:
//!
//! * Every entity has a **locally minted, deterministic opaque id** derived from its natural key.
//!   No external vendor id is ever the canonical identity; vendor ids live in
//!   [`SourceIdentity`] lists and can disappear without invalidating a canonical record.
//! * Cohort membership is [`GradYear`] — an absolute, immutable property. Grade level is never a
//!   cohort key; it is recorded as [`ObservedGrade`] together with the school year and source that
//!   observed it, and only *deterministically implies* a [`GradYear`].
//! * Events are described by our own ontology ([`EventKind`]); vendor event names are source evidence
//!   ([`SourceEventLabel`]), not canonical keys.

use crate::jurisdiction::UsJurisdiction;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

mod records;
mod review;

pub use records::{
    AccessBlockKind, CollectionSnapshot, CoverageRow, CoverageScope, RetainedConflict, ReviewCase,
    ReviewState, SourceAccessCondition, SourceEntityKind, SourceMeetRef, SourceObjectIdentity,
};
pub use review::{
    ReviewCaseFact, ReviewEvidenceFact, ReviewPacket, ReviewVerdict, ReviewVerdictKind,
    ReviewVerdictRecord, VerdictBatch,
};

// -------------------------------------------------------------------------------------------------
// Identifiers
// -------------------------------------------------------------------------------------------------

/// Marker types for [`Id`] tags.
pub mod tag {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct School;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Team;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Coach;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Athlete;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Meet;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Event;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Performance;
}

/// A canonical, locally minted identifier: `<prefix>_<16 lowercase hex chars>`.
///
/// The value is the first 64 bits of `SHA-256(prefix || '\u{1f}' || natural key parts…)`. It is
/// deterministic across runs and machines, so re-running a collector against the same evidence
/// yields the same canonical ids, while remaining independent of any external vendor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<T> {
    value: String,
    #[serde(skip)]
    _tag: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn mint(prefix: &str, parts: &[&str]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(prefix.as_bytes());
        for part in parts {
            hasher.update([0x1f]);
            hasher.update(part.as_bytes());
        }
        let digest = hasher.finalize();
        let mut hex = String::with_capacity(17 + 16);
        hex.push_str(prefix);
        hex.push('_');
        // SHA-256 always yields 32 bytes; `take(8)` keeps the 64-bit identity prefix.
        for byte in digest.iter().take(8) {
            hex.push_str(&format!("{byte:02x}"));
        }
        Self {
            value: hex,
            _tag: PhantomData,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

pub type SchoolId = Id<tag::School>;
pub type TeamId = Id<tag::Team>;
pub type CoachId = Id<tag::Coach>;
pub type AthleteId = Id<tag::Athlete>;
pub type MeetId = Id<tag::Meet>;
pub type EventId = Id<tag::Event>;
pub type PerformanceId = Id<tag::Performance>;

// -------------------------------------------------------------------------------------------------
// Time, grade, cohort
// -------------------------------------------------------------------------------------------------

/// A school year, identified by its starting calendar year (2025 = "2025-26").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchoolYear(pub i16);

impl SchoolYear {
    pub const fn start_year(self) -> i16 {
        self.0
    }

    /// `"2025-26"`.
    pub fn short(self) -> String {
        // `%` is a truncating remainder, which `wrapping_rem` reproduces exactly (including for
        // negative years) without the `MIN % -1` panic; `saturating_add` keeps the value
        // well-defined at `i16::MAX` instead of panicking in debug builds.
        let end = self.0.saturating_add(1).wrapping_rem(100);
        format!("{}-{end:02}", self.0)
    }

    /// The school year that contains `date` for competition purposes (Aug 1 boundary).
    pub fn containing(year: i16, month: u8) -> Self {
        if month >= 8 {
            SchoolYear(year)
        } else {
            // Saturating: a year at `i16::MIN` clamps instead of panicking in debug builds.
            SchoolYear(year.saturating_sub(1))
        }
    }
}

/// Grade level as observed in a specific school year: 9..=12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Grade(u8);

impl Grade {
    pub fn new(grade: u8) -> Option<Self> {
        (9..=12).contains(&grade).then_some(Grade(grade))
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An absolute graduating class, e.g. `GradYear(2027)` = "Class of 2027".
///
/// This is the only sanctioned cohort key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GradYear(pub i16);

impl GradYear {
    /// Class of 2027.
    pub const CO2027: GradYear = GradYear(2027);

    /// Raw graduation year, e.g. `2027`. The tuple field stays public for pattern matching, the
    /// accessor is what domain code should read so a future invariant change lands in one place.
    pub const fn get(self) -> i16 {
        self.0
    }

    pub fn new(year: i16) -> Option<Self> {
        (2020..=2040).contains(&year).then_some(GradYear(year))
    }

    /// `grade` observed during `school_year` implies `grad_year = start + (13 - grade)`.
    ///
    /// Grade 11 in 2025-26 -> 2027. Grade 12 in 2026-27 -> 2027. Grade 9 in 2025-26 -> 2029.
    pub fn of(grade: Grade, school_year: SchoolYear) -> Self {
        // In-domain inputs (4-digit years, grades 9..=12) stay far inside `i16`; saturating keeps
        // out-of-domain inputs well-defined instead of panicking in debug builds.
        GradYear(
            school_year
                .start_year()
                .saturating_add(13)
                .saturating_sub(i16::from(grade.get())),
        )
    }
}

impl fmt::Display for GradYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A grade observation: grade + the school year it was observed in + where it came from.
///
/// Never collapse this into a bare `grade`, and never overwrite it when a newer observation
/// arrives: history is evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedGrade {
    pub grade: Grade,
    pub school_year: SchoolYear,
    pub source: SourceRef,
}

impl ObservedGrade {
    pub fn grad_year(&self) -> GradYear {
        GradYear::of(self.grade, self.school_year)
    }
}

// -------------------------------------------------------------------------------------------------
// Provenance
// -------------------------------------------------------------------------------------------------

/// A registered data source. `id` is a stable slug used in evidence and reports.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceRef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SourceRef {
    pub fn new(id: impl Into<String>, url: Option<String>) -> Self {
        Self { id: id.into(), url }
    }

    pub fn id(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            url: None,
        }
    }
}

/// How a fact was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMethod {
    /// Read directly from a fetched document (HTML/JSON/PDF/CSV).
    Fetched,
    /// Extracted by parsing a fetched document.
    Parsed,
    /// Deterministically derived from other evidence (e.g. grade + school year -> grad year).
    Derived,
    /// Asserted by an upstream dataset that is itself under evidence (legacy import).
    Inherited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub source: SourceRef,
    pub method: EvidenceMethod,
    /// ISO-8601 date the source was observed.
    pub observed_on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Evidence {
    pub fn fetched(source: SourceRef, observed_on: impl Into<String>) -> Self {
        Self {
            source,
            method: EvidenceMethod::Fetched,
            observed_on: observed_on.into(),
            note: None,
        }
    }

    pub fn parsed(source: SourceRef, observed_on: impl Into<String>) -> Self {
        Self {
            source,
            method: EvidenceMethod::Parsed,
            observed_on: observed_on.into(),
            note: None,
        }
    }

    pub fn derived(
        source: SourceRef,
        observed_on: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            source,
            method: EvidenceMethod::Derived,
            observed_on: observed_on.into(),
            note: Some(note.into()),
        }
    }
}

/// Namespace of an external identity. Namespaces are open-ended by design (`Other("...")`) so new
/// providers never force a model change; the well-known ones are enumerated for type safety.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceNamespace {
    MilesplitSchool,
    MilesplitTeam,
    MilesplitAthlete,
    MilesplitMeet,
    TfrrsTeam,
    TfrrsAthlete,
    TfrrsMeet,
    DirectAthleticsTeam,
    DirectAthleticsAthlete,
    /// `association` is the state association slug (`wiaa`, `ihsa`, `kshsaa`, …).
    AssociationSchool {
        association: String,
    },
    AssociationAthlete {
        association: String,
    },
    TimerTeam {
        provider: String,
    },
    TimerAthlete {
        provider: String,
    },
    TimerMeet {
        provider: String,
    },
    /// Historical Athletic.net-derived ids retained as legacy evidence only.
    LegacyAthleticNet {
        kind: String,
    },
    /// Athletic.net ids read through the owner-authorized athlete-bio adapter (`athlete`, `school`,
    /// `meet`). Non-core: it is the same vendor the platform's own adapters exist to be
    /// independent of.
    AthleticNet {
        kind: String,
    },
    Other(String),
}

impl SourceNamespace {
    /// True when the namespace is supplied by one of the platform's own adapters rather than by
    /// Athletic.net or its mirror.
    ///
    /// Only the Athletic.net namespaces are non-core by definition. Timer namespaces stay core: the
    /// AthleticLIVE-derived rows that carry them are already excluded by their evidence source id,
    /// while a real timing provider (`pttiming`, `wayzata`, …) is a core source.
    pub fn is_core(&self) -> bool {
        !matches!(
            self,
            SourceNamespace::LegacyAthleticNet { .. } | SourceNamespace::AthleticNet { .. }
        )
    }
}

impl fmt::Display for SourceNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceNamespace::MilesplitSchool => f.write_str("milesplit_school"),
            SourceNamespace::MilesplitTeam => f.write_str("milesplit_team"),
            SourceNamespace::MilesplitAthlete => f.write_str("milesplit_athlete"),
            SourceNamespace::MilesplitMeet => f.write_str("milesplit_meet"),
            SourceNamespace::TfrrsTeam => f.write_str("tfrrs_team"),
            SourceNamespace::TfrrsAthlete => f.write_str("tfrrs_athlete"),
            SourceNamespace::TfrrsMeet => f.write_str("tfrrs_meet"),
            SourceNamespace::DirectAthleticsTeam => f.write_str("direct_athletics_team"),
            SourceNamespace::DirectAthleticsAthlete => f.write_str("direct_athletics_athlete"),
            SourceNamespace::AssociationSchool { association } => {
                write!(f, "association_school:{association}")
            }
            SourceNamespace::AssociationAthlete { association } => {
                write!(f, "association_athlete:{association}")
            }
            SourceNamespace::TimerTeam { provider } => write!(f, "timer_team:{provider}"),
            SourceNamespace::TimerAthlete { provider } => write!(f, "timer_athlete:{provider}"),
            SourceNamespace::TimerMeet { provider } => write!(f, "timer_meet:{provider}"),
            SourceNamespace::LegacyAthleticNet { kind } => {
                write!(f, "legacy_athletic_net:{kind}")
            }
            SourceNamespace::AthleticNet { kind } => write!(f, "athleticnet:{kind}"),
            SourceNamespace::Other(value) => f.write_str(value),
        }
    }
}

/// An identity this entity carries in some external system.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub namespace: SourceNamespace,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SourceIdentity {
    pub fn new(namespace: SourceNamespace, id: impl Into<String>) -> Self {
        Self {
            namespace,
            id: id.into(),
            url: None,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

/// Confidence in an identity merge or field value, 0..=100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Confidence(pub u8);

impl Confidence {
    pub const HIGH: Confidence = Confidence(85);
    pub const MEDIUM: Confidence = Confidence(65);
    pub const LOW: Confidence = Confidence(40);

    pub fn new(value: u8) -> Self {
        Confidence(value.min(100))
    }
}

// -------------------------------------------------------------------------------------------------
// Classification
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Boys,
    Girls,
    Mixed,
    Unknown,
}

impl Gender {
    pub fn parse_milesplit(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "m" | "male" | "boys" | "boy" => Gender::Boys,
            "f" | "female" | "girls" | "girl" => Gender::Girls,
            _ => Gender::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sport {
    OutdoorTrack,
    IndoorTrack,
    CrossCountry,
}

/// A team is (school, sport, gender side, season).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTeam {
    pub id: TeamId,
    pub school: SchoolId,
    pub sport: Sport,
    pub gender: Gender,
    pub school_year: SchoolYear,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
}

impl CanonicalTeam {
    /// One team per (school, sport, gender side, school year). The same four values always mint the
    /// same id, so a provider re-publishing a roster upserts the team instead of duplicating it.
    pub fn mint(
        school: &SchoolId,
        sport: Sport,
        gender: Gender,
        school_year: SchoolYear,
    ) -> TeamId {
        Id::mint(
            "team",
            &[
                school.as_str(),
                &format!("{sport:?}"),
                &format!("{gender:?}"),
                &school_year.start_year().to_string(),
            ],
        )
    }
}

/// Meet competition level, from our own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionLevel {
    Invitational,
    Dual,
    Conference,
    District,
    Regional,
    Sectional,
    State,
    National,
    Unknown,
}

// -------------------------------------------------------------------------------------------------
// Event ontology
// -------------------------------------------------------------------------------------------------

/// Our own event taxonomy. Vendor strings map into this via [`EventKind::from_source_label`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Track100m,
    Track200m,
    Track400m,
    Track800m,
    Track1600m,
    Track3200m,
    Track1Mile,
    Track3000m,
    Track5000m,
    Track110mHurdles,
    Track100mHurdles,
    Track300mHurdles,
    Track400mHurdles,
    Track2000mSteeplechase,
    Track3000mSteeplechase,
    /// Cross-country race; the published distance varies by division and course.
    CrossCountry,
    Relay4x100,
    Relay4x200,
    Relay4x400,
    Relay4x800,
    SprintMedley,
    DistanceMedley,
    HighJump,
    LongJump,
    TripleJump,
    PoleVault,
    ShotPut,
    Discus,
    Javelin,
    Hammer,
    WeightThrow,
    Pentathlon,
    Heptathlon,
    Decathlon,
    /// Recognized source label that has no canonical home yet.
    Unmapped {
        label: String,
    },
}

impl EventKind {
    /// Map a source label (`"1600m"`, `"110mH"`, `"Shot Put"`, `"4x400m Relay"`, …) to the ontology.
    ///
    /// Unknown labels are preserved as [`EventKind::Unmapped`] rather than dropped.
    pub fn from_source_label(label: &str) -> Self {
        let compact = Self::normalized_label(label);
        match compact.as_str() {
            "100m" => EventKind::Track100m,
            "200m" => EventKind::Track200m,
            "400m" => EventKind::Track400m,
            "800m" => EventKind::Track800m,
            "1600m" => EventKind::Track1600m,
            "3200m" => EventKind::Track3200m,
            "1mile" | "mile" => EventKind::Track1Mile,
            "3000m" | "3k" => EventKind::Track3000m,
            "5000m" | "5k" => EventKind::Track5000m,
            "110mh" | "110h" | "110mhurdles" | "110mhhurdles" => EventKind::Track110mHurdles,
            "100mh" | "100h" | "100mhurdles" => EventKind::Track100mHurdles,
            "300mh" | "300h" | "300mhurdles" | "300mhhurdles" => EventKind::Track300mHurdles,
            "400mh" | "400h" => EventKind::Track400mHurdles,
            "2000msteeplechase" | "2ksteeplechase" | "2000msteeple" => {
                EventKind::Track2000mSteeplechase
            }
            "3000msteeplechase" | "3ksteeplechase" | "3000msteeple" => {
                EventKind::Track3000mSteeplechase
            }
            "crosscountry" | "xc" | "crosscountryrace" => EventKind::CrossCountry,
            "4x100m" | "4x100" | "4x100mrelay" | "4x100relay" | "400mrelay" => {
                EventKind::Relay4x100
            }
            "4x200m" | "4x200" | "4x200mrelay" | "4x200relay" | "800mrelay" => {
                EventKind::Relay4x200
            }
            "4x400m" | "4x400" | "4x400mrelay" | "4x400relay" | "1600mrelay" => {
                EventKind::Relay4x400
            }
            "4x800m" | "4x800" | "4x800mrelay" | "4x800relay" | "3200mrelay" => {
                EventKind::Relay4x800
            }
            "sprintmedley" | "smed" | "smr" => EventKind::SprintMedley,
            "distancemedley" | "dmed" | "dmr" => EventKind::DistanceMedley,
            "highjump" | "hj" => EventKind::HighJump,
            "longjump" | "lj" => EventKind::LongJump,
            "triplejump" | "tj" => EventKind::TripleJump,
            "polevault" | "pv" => EventKind::PoleVault,
            "shotput" | "shot" | "sp" => EventKind::ShotPut,
            "discus" | "disc" => EventKind::Discus,
            "javelin" | "jav" | "jt" => EventKind::Javelin,
            "hammer" | "ht" => EventKind::Hammer,
            "weightthrow" | "wt" => EventKind::WeightThrow,
            "pentathlon" => EventKind::Pentathlon,
            "heptathlon" => EventKind::Heptathlon,
            "decathlon" => EventKind::Decathlon,
            _ => EventKind::Unmapped {
                label: label.trim().to_string(),
            },
        }
    }

    /// Fold a source label to its compact form: no spaces, `-` or `_`, lowercase, and
    /// `meters`/`metre`/`meter` all read as `m`.
    fn normalized_label(label: &str) -> String {
        let normalized: String = label
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
            .collect::<String>()
            .to_ascii_lowercase();
        normalized
            .replace("meters", "m")
            .replace("metre", "m")
            .replace("meter", "m")
    }

    pub fn is_field(&self) -> bool {
        matches!(
            self,
            EventKind::HighJump
                | EventKind::LongJump
                | EventKind::TripleJump
                | EventKind::PoleVault
                | EventKind::ShotPut
                | EventKind::Discus
                | EventKind::Javelin
                | EventKind::Hammer
                | EventKind::WeightThrow
                | EventKind::Pentathlon
                | EventKind::Heptathlon
                | EventKind::Decathlon
        )
    }

    pub fn is_relay(&self) -> bool {
        matches!(
            self,
            EventKind::Relay4x100
                | EventKind::Relay4x200
                | EventKind::Relay4x400
                | EventKind::Relay4x800
                | EventKind::SprintMedley
                | EventKind::DistanceMedley
        )
    }
}

/// A raw event label seen at a source, retained as evidence next to the mapped [`EventKind`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEventLabel {
    pub source: SourceRef,
    pub label: String,
}

// -------------------------------------------------------------------------------------------------
// Canonical entities
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSchool {
    pub id: SchoolId,
    pub name: String,
    pub normalized_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<UsJurisdiction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub association: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrollment: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub school_website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub athletics_website: Option<String>,
    pub co_op: bool,
    pub aliases: Vec<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
}

impl CanonicalSchool {
    /// Build a school from its natural key (jurisdiction + normalized name) so that id minting is
    /// deterministic and identical no matter which adapter saw the school first.
    ///
    /// The key carries the jurisdiction's USPS code — byte-identical to the uppercase state string
    /// this parameter used to hold — so typing the parameter re-mints no school id.
    pub fn mint(state: UsJurisdiction, name: &str, normalized_name: &str) -> SchoolId {
        let _ = name;
        // The key is the normalized name with whitespace/punctuation removed. Providers publish the
        // same school both as a display name ("Aberdeen Central") and as a URL slug
        // ("aberdeencentral"), so the compressed form is what makes those two observations mint one
        // canonical school instead of two.
        let compressed: String = normalized_name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        Id::mint("sch", &[state.code(), &compressed])
    }

    pub fn new(
        state: UsJurisdiction,
        name: impl Into<String>,
        normalized_name: impl Into<String>,
    ) -> (Self, SchoolId) {
        let name = name.into();
        let normalized_name = normalized_name.into();
        let id = CanonicalSchool::mint(state, &name, &normalized_name);
        (
            Self {
                id: id.clone(),
                name,
                normalized_name,
                city: None,
                state: Some(state),
                association: None,
                classification: None,
                enrollment: None,
                school_website: None,
                athletics_website: None,
                co_op: false,
                aliases: Vec::new(),
                source_identities: Vec::new(),
                evidence: Vec::new(),
            },
            id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalCoach {
    pub id: CoachId,
    pub name: String,
    pub school: SchoolId,
    /// `None` for school-wide roles (athletic director) that are not bound to a single sport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: CoachRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub professional_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    /// Set by the merge when a consumer mailbox was dropped from `professional_email`. Not
    /// serialized: it is derived from the observation bodies on every read, so re-importing the
    /// entity log re-derives it. A withheld row serializes without the key, where the pre-Fjall
    /// pipeline wrote an explicit `null`.
    #[serde(skip)]
    pub email_withheld: bool,
}

impl CanonicalCoach {
    pub fn new(
        school: &SchoolId,
        name: impl Into<String>,
        sport: Option<Sport>,
        gender: Gender,
        role: CoachRole,
    ) -> Self {
        let name = name.into();
        let id = Id::mint(
            "coa",
            &[
                school.as_str(),
                &normalize_name(&name),
                &format!("{sport:?}"),
                &format!("{gender:?}"),
                &format!("{role:?}"),
            ],
        );
        Self {
            id,
            name,
            school: school.clone(),
            sport,
            gender,
            role,
            professional_email: None,
            phone: None,
            source_identities: Vec::new(),
            evidence: Vec::new(),
            email_withheld: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachRole {
    HeadCoach,
    AssistantCoach,
    AthleticDirector,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAthlete {
    pub id: AthleteId,
    pub canonical_name: String,
    pub known_names: Vec<String>,
    pub grad_year: GradYear,
    pub school: SchoolId,
    pub gender: Gender,
    pub sports: Vec<Sport>,
    /// Grade observations, newest last; never collapsed into `grad_year` alone.
    pub observed_grades: Vec<ObservedGrade>,
    pub public_profile_urls: Vec<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    pub identity_confidence: Confidence,
}

impl CanonicalAthlete {
    /// Mint an athlete from (school, normalized name, grad year, gender).
    ///
    /// Two sources that agree on those four facts produce the same canonical athlete without any
    /// shared vendor id.
    pub fn mint(school: &SchoolId, name: &str, grad_year: GradYear, gender: Gender) -> AthleteId {
        Id::mint(
            "ath",
            &[
                school.as_str(),
                &normalize_name(name),
                &grad_year.0.to_string(),
                match gender {
                    Gender::Boys => "m",
                    Gender::Girls => "f",
                    _ => "u",
                },
            ],
        )
    }

    pub fn new(
        school: &SchoolId,
        name: impl Into<String>,
        grad_year: GradYear,
        gender: Gender,
    ) -> Self {
        let canonical_name = name.into();
        let id = CanonicalAthlete::mint(school, &canonical_name, grad_year, gender);
        Self {
            id,
            canonical_name: canonical_name.clone(),
            known_names: vec![canonical_name],
            grad_year,
            school: school.clone(),
            gender,
            sports: Vec::new(),
            observed_grades: Vec::new(),
            public_profile_urls: Vec::new(),
            source_identities: Vec::new(),
            evidence: Vec::new(),
            identity_confidence: Confidence::MEDIUM,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalMeet {
    pub id: MeetId,
    pub name: String,
    pub normalized_name: String,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// The jurisdiction the meet was held in.
    ///
    /// `None` is a *coverage gap*, not an unknown state: no evidence placed the venue in any
    /// jurisdiction, and [`MEET_STATE_UNRESOLVED`] is how a report spells that bucket. Both
    /// directions keep the free-string wire form - `Some` is the jurisdiction's code, so a placed
    /// meet still writes `"state":"WI"`, and `None` is the sentinel, so an unplaced one still writes
    /// `"state":"??"` instead of dropping the key - and the sentinel decodes back to `None`, so a
    /// meet row written before this field was typed still reads.
    #[serde(
        default,
        serialize_with = "serialize_meet_state",
        deserialize_with = "deserialize_meet_state"
    )]
    pub state: Option<UsJurisdiction>,
    pub level: CompetitionLevel,
    pub sports: Vec<Sport>,
    pub source_identities: Vec<SourceIdentity>,
    pub source_urls: Vec<String>,
    pub evidence: Vec<Evidence>,
}

/// The pre-cutover wire sentinel for a meet whose venue was never placed in a jurisdiction.
///
/// Meet identity hashes it exactly as the free-string era hashed the string `"??"`, so every meet
/// already stored under that sentinel keeps its id; a report renders the unresolved bucket with it.
pub const MEET_STATE_UNRESOLVED: &str = "??";

/// Write a meet's `state` in the form the store has always carried: code, or the sentinel.
///
/// The key is never omitted. `skip_serializing_if` would shrink an unplaced meet's row by one line,
/// which is a silent wire change for every reader that counts keys and a visible one for the golden
/// captures, so `None` is written as [`MEET_STATE_UNRESOLVED`] exactly as the free-form era wrote it.
fn serialize_meet_state<S>(state: &Option<UsJurisdiction>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(state.map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code))
}

/// Decode a meet's `state`: the legacy [`MEET_STATE_UNRESOLVED`] sentinel or a jurisdiction.
///
/// This is the one place the sentinel is legal. Everything downstream sees
/// `Option<UsJurisdiction>`, so no reader can mistake "never placed" for a jurisdiction named `??`,
/// while a genuinely unknown code stays a decode error instead of a retained string. An absent key
/// never reaches this function: the field's `default` supplies `None` for it.
fn deserialize_meet_state<'de, D>(deserializer: D) -> Result<Option<UsJurisdiction>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    match raw.as_str() {
        MEET_STATE_UNRESOLVED => Ok(None),
        code => UsJurisdiction::parse(code).map(Some).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "meet state {code:?} is not one of the 50 states or the District of Columbia"
            ))
        }),
    }
}

impl CanonicalMeet {
    /// Meet identity = jurisdiction + date + normalized name.
    ///
    /// Location is deliberately excluded: providers spell the same venue differently ("UW-La Crosse"
    /// vs "La Crosse, WI"), and a meet that one source publishes with a location and another without
    /// must still be one canonical meet. The observed location is retained on the record as a field.
    ///
    /// The key carries the jurisdiction's USPS code — byte-identical to the uppercase state string
    /// this parameter used to hold — and [`MEET_STATE_UNRESOLVED`] when the venue was never placed,
    /// which is the string the free-form era hashed, so typing the parameter re-mints no meet id.
    pub fn mint(
        state: Option<UsJurisdiction>,
        date: &str,
        name: &str,
        _location: Option<&str>,
    ) -> MeetId {
        Id::mint(
            "meet",
            &[
                state
                    .map(UsJurisdiction::code)
                    .unwrap_or(MEET_STATE_UNRESOLVED),
                date,
                &normalize_name(name),
            ],
        )
    }

    pub fn new(
        state: Option<UsJurisdiction>,
        name: impl Into<String>,
        date: impl Into<String>,
        level: CompetitionLevel,
    ) -> Self {
        let name = name.into();
        let date = date.into();
        let normalized_name = normalize_name(&name);
        let id = CanonicalMeet::mint(state, &date, &name, None);
        Self {
            id,
            name,
            normalized_name,
            date,
            end_date: None,
            location: None,
            state,
            level,
            sports: Vec::new(),
            source_identities: Vec::new(),
            source_urls: Vec::new(),
            evidence: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalEvent {
    pub id: EventId,
    pub meet: MeetId,
    pub kind: EventKind,
    pub gender: Gender,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub division: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<String>,
    pub source_labels: Vec<SourceEventLabel>,
    pub evidence: Vec<Evidence>,
}

impl CanonicalEvent {
    pub fn new(
        meet: &MeetId,
        kind: EventKind,
        gender: Gender,
        division: Option<&str>,
        round: Option<&str>,
    ) -> Self {
        let id = Id::mint(
            "evt",
            &[
                meet.as_str(),
                &format!("{kind:?}"),
                match gender {
                    Gender::Boys => "m",
                    Gender::Girls => "f",
                    _ => "u",
                },
                division.unwrap_or(""),
                round.unwrap_or(""),
            ],
        );
        Self {
            id,
            meet: meet.clone(),
            kind,
            gender,
            division: division.map(str::to_string),
            round: round.map(str::to_string),
            source_labels: Vec::new(),
            evidence: Vec::new(),
        }
    }
}

/// A mark's unit context. Track marks are times (or points for combined events), field marks are
/// distances/heights in metric or imperial notation, exactly as published by the source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mark {
    /// Seconds (e.g. `10.94`, `4:41.23` already converted to 281.23).
    TimeSeconds(f64),
    /// Metres, converted from the published imperial/metric value.
    DistanceMetres(f64),
    /// A field mark preserved in the source's own notation (e.g. `5' 4"`, `42-06.5`).
    FieldImperial { feet_mark: String, metres: f64 },
    /// Combined-event or team points.
    Points(f64),
    /// Published verbatim, not yet parsed.
    Raw(String),
}

impl Mark {
    pub fn raw(&self) -> &str {
        match self {
            Mark::Raw(value) => value,
            Mark::TimeSeconds(_) => "time",
            Mark::DistanceMetres(_) => "distance",
            Mark::FieldImperial { .. } => "field",
            Mark::Points(_) => "points",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalPerformance {
    pub id: PerformanceId,
    pub athlete: AthleteId,
    pub team: TeamId,
    pub event: EventId,
    pub meet: MeetId,
    pub date: String,
    pub mark: Mark,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind_mps: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<TimingMethod>,
    /// The athlete's grade at the moment of this performance, when the source publishes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_grade: Option<Grade>,
    pub evidence: Vec<Evidence>,
    /// Provider-local result key, used for idempotent upserts.
    pub source_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingMethod {
    Fat,
    Hand,
    Unknown,
}

impl CanonicalPerformance {
    pub fn mint(
        athlete: &AthleteId,
        meet: &MeetId,
        kind: &EventKind,
        date: &str,
        source_key: &str,
    ) -> PerformanceId {
        Id::mint(
            "perf",
            &[
                athlete.as_str(),
                meet.as_str(),
                &format!("{kind:?}"),
                date,
                source_key,
            ],
        )
    }
}

// -------------------------------------------------------------------------------------------------
// Contact policy
// -------------------------------------------------------------------------------------------------

/// Consumer mailboxes. An address on one of these domains is a personal mailbox rather than a
/// school/sport contact, so the collection contract drops it wherever a provider publishes one.
pub const CONSUMER_MAIL_DOMAINS: [&str; 12] = [
    "gmail.com",
    "googlemail.com",
    "hotmail.com",
    "outlook.com",
    "live.com",
    "msn.com",
    "yahoo.com",
    "aol.com",
    "icloud.com",
    "me.com",
    "protonmail.com",
    "proton.me",
];

/// Keep a published address only when it is not a consumer mailbox; `None` for a malformed address
/// or a personal mailbox.
pub fn professional_email(address: &str) -> Option<String> {
    let address = address.trim();
    let (local, domain) = address.split_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }
    let domain = domain.to_ascii_lowercase();
    let consumer = CONSUMER_MAIL_DOMAINS.iter().any(|base| {
        domain == *base
            || domain
                .strip_suffix(base)
                .is_some_and(|prefix| prefix.ends_with('.'))
    });
    (!consumer).then(|| address.to_string())
}

// -------------------------------------------------------------------------------------------------
// Normalization
// -------------------------------------------------------------------------------------------------

/// Normalize a person/school/meet name for identity comparisons: lowercase, strip diacritics and
/// punctuation, collapse whitespace, drop school-type suffixes that vary between sources
/// ("high school", "hs", "school", "academy" is *kept* because it is distinguishing).
///
/// Suffixes are dropped until none applies, so the result is a fixpoint: normalizing an
/// already-normalized name is the identity. `SchoolId::mint` keys the identity on this string, so
/// two adapters handing the same school over in different spellings have to agree even when one of
/// them re-normalizes a name the other passed through raw.
pub fn normalize_name(raw: &str) -> String {
    let lowered = raw.trim().to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if ch.is_whitespace()
            || ch == '-'
            || ch == '\''
            || ch == '.'
            || ch == ','
            || ch == '/'
        {
            out.push(' ');
        } else if !ch.is_ascii() {
            if let Some(replacement) = strip_diacritic(ch) {
                out.push(replacement);
            }
        }
    }
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut parts: Vec<&str> = collapsed.split(' ').collect();
    while strip_type_suffix(&mut parts) {}
    parts.join(" ")
}

/// Strip one trailing school-type suffix from `parts`, reporting whether anything was removed.
///
/// One call removes at most one suffix; [`normalize_name`] calls this until it returns `false`, so
/// a name that ends in several of them ("X High School School") reaches the same key as the name
/// without them instead of stopping one suffix short.
fn strip_type_suffix(parts: &mut Vec<&str>) -> bool {
    for suffix in [
        "high school",
        "hs",
        "highschool",
        "school",
        "sr high",
        "senior high",
    ] {
        let tokens: Vec<&str> = suffix.split(' ').collect();
        if parts.len() > tokens.len() && parts.ends_with(&tokens) {
            // Guarded above (`parts.len() > tokens.len()`), so saturating is exact here.
            parts.truncate(parts.len().saturating_sub(tokens.len()));
            return true;
        }
    }
    false
}

fn strip_diacritic(ch: char) -> Option<char> {
    // A tiny, deterministic folding table is enough for Midwest school/person names; anything else
    // falls back to the unaccented ASCII range when possible.
    let folded = match ch {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ę' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ī' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' | 'ø' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'ū' => 'u',
        'ñ' | 'ń' => 'n',
        'ç' | 'ć' => 'c',
        'š' | 'ś' => 's',
        'ž' | 'ź' | 'ż' => 'z',
        'ý' | 'ÿ' => 'y',
        'ł' => 'l',
        'æ' => 'a',
        'œ' => 'o',
        'ß' => 's',
        _ => return None,
    };
    Some(folded)
}

/// Sort a `"Last, First"` roster name into `"First Last"`; leave other shapes alone.
pub fn flip_last_first(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some((last, first)) = trimmed.split_once(',') {
        let last = last.trim();
        let first = first.trim();
        if !last.is_empty() && !first.is_empty() {
            return format!("{first} {last}");
        }
    }
    trimmed.to_string()
}

/// Convenience map for counters used by adapter reports.
pub type Counters = BTreeMap<String, u64>;

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
