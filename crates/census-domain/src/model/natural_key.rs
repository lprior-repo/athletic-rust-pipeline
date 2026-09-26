//! Natural-key material: what a canonical row's id was minted from, restated by the row itself.
//!
//! [`Id::mint`](super::Id::mint) keeps the first 64 bits of SHA-256, so two different subjects can in
//! principle land on one canonical id. The store keys rows by id and merges on that key, which turns
//! such a collision into a silent merge of two subjects — unless the rows can state the material
//! their id came from and the merge compares it before absorbing anything. That is what [`NaturalKey`]
//! is for, and why the comparison runs over already-decoded fields: the merge runs once per
//! observation on every read of the store, so a check that re-hashed the material would put a second
//! SHA-256 per observation on the read path.
//!
//! [`collision::id_collision`](super::collision::id_collision) turns one such comparison into the
//! retained finding the store's `conflicts` table holds: it names the id, both sides' material and
//! both sides' sources, so an operator acts on the two subjects rather than on a count of them.

use super::collision::{evidence_list, source_list};
use super::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, RetainedConflict,
};
use crate::jurisdiction::UsJurisdiction;

/// A canonical row that mints its id from a natural key and can restate that key from its own fields.
///
/// Implementors mirror their `mint` call exactly. Where a `mint` part is derived — a normalized name,
/// a compressed name, a gender side — the derivation is repeated here rather than compared raw, so two
/// sources that spell one subject differently are one subject and not a collision.
pub trait NaturalKey {
    /// Whether `other` states the same material this row's id was minted from.
    ///
    /// Cheap by contract: already-decoded fields, no hashing, and byte-identical names short-circuit
    /// before any normalization, so re-reading an observation the store already holds costs field
    /// comparisons and nothing else.
    fn same_natural_key(&self, other: &Self) -> bool;

    /// The material as a retained conflict prints it: what this row's id was minted from.
    ///
    /// Only the collision path calls this, so it may format freely.
    fn natural_key(&self) -> String;

    /// The sources a collision has to name for this row: its provider identities where the row keeps
    /// a list of them, otherwise the sources its evidence came from. A row no source has claimed
    /// renders as `-`.
    fn sources(&self) -> String;

    /// The canonical-id collisions this row's merge retained. The store's `conflicts` table holds one
    /// row per id, so a reader that drains this field files one finding however often the merge ran.
    fn retained_conflicts(&self) -> &[RetainedConflict];
}

/// Whether two names mint one canonical name: identical raw names short-circuit, so the
/// duplicate-observation path never normalizes, and only a real disagreement pays for the two
/// normalizations that decide it.
fn same_name(left: &str, right: &str) -> bool {
    left == right || normalize_name(left) == normalize_name(right)
}

/// Whether two normalized names compress to the same key: school identity drops every non-alphanumeric
/// character, and comparing the filtered characters keeps that allocation-free on the merge path.
fn same_compressed(left: &str, right: &str) -> bool {
    left.chars()
        .filter(char::is_ascii_alphanumeric)
        .eq(right.chars().filter(char::is_ascii_alphanumeric))
}

/// The compressed form [`same_compressed`] compares, materialized for the conflict line.
fn compressed(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphanumeric).collect()
}

impl NaturalKey for CanonicalSchool {
    /// `SchoolId::mint` hashes the jurisdiction's code and the normalized name with every
    /// non-alphanumeric character removed. The display name is deliberately not part of the key — one
    /// school is published both as "Aberdeen Central" and as the slug "aberdeencentral" — so a
    /// difference there is not a collision.
    fn same_natural_key(&self, other: &Self) -> bool {
        self.state == other.state && same_compressed(&self.normalized_name, &other.normalized_name)
    }

    fn natural_key(&self) -> String {
        let state = self.state.map_or("??", UsJurisdiction::code);
        format!(
            "school in {state} named {:?}",
            compressed(&self.normalized_name)
        )
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalTeam {
    /// `TeamId::mint` hashes the four values a team *is*: school, sport, gender side and season. Each
    /// is already a distinct value on the row, so the comparison reads them directly.
    fn same_natural_key(&self, other: &Self) -> bool {
        self.school == other.school
            && self.sport == other.sport
            && self.gender == other.gender
            && self.school_year == other.school_year
    }

    fn natural_key(&self) -> String {
        format!(
            "team {:?} {:?} at {} in {}",
            self.sport,
            self.gender,
            self.school,
            self.school_year.get()
        )
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalCoach {
    /// `CoachId::mint` hashes the school, the normalized name, and the sport/gender/role the coach
    /// holds there. Two rows that agree on all five are one coach however the name is spelled.
    fn same_natural_key(&self, other: &Self) -> bool {
        self.school == other.school
            && self.sport == other.sport
            && self.gender == other.gender
            && self.role == other.role
            && same_name(&self.name, &other.name)
    }

    fn natural_key(&self) -> String {
        format!(
            "coach {:?} at {} ({:?} {:?} {:?})",
            normalize_name(&self.name),
            self.school,
            self.sport,
            self.gender,
            self.role
        )
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalAthlete {
    /// A subject is bound to the provider's object as well as the candidate-search facts.
    fn same_natural_key(&self, other: &Self) -> bool {
        self.school == other.school
            && self.grad_year == other.grad_year
            && self.gender == other.gender
            && same_name(&self.canonical_name, &other.canonical_name)
            && self.source.namespace == other.source.namespace
            && self.source.id == other.source.id
    }

    fn natural_key(&self) -> String {
        format!(
            "athlete {:?} at {} (class {}, {:?})",
            normalize_name(&self.canonical_name),
            self.school,
            self.grad_year.get(),
            self.gender
        )
    }

    fn sources(&self) -> String {
        source_list(self.identities())
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalMeet {
    /// `MeetId::mint` hashes the jurisdiction (or the unresolved sentinel), the date and the
    /// normalized name. The location is excluded on purpose: the same meet is published with and
    /// without a venue spelling, and both observations are one meet.
    fn same_natural_key(&self, other: &Self) -> bool {
        self.state == other.state && self.date == other.date && same_name(&self.name, &other.name)
    }

    fn natural_key(&self) -> String {
        let state = self
            .state
            .map_or(super::MEET_STATE_UNRESOLVED, UsJurisdiction::code);
        format!(
            "meet in {state} named {:?} on {}",
            normalize_name(&self.name),
            self.date
        )
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalEvent {
    /// `EventId::mint` hashes the meet, the kind, the gender side, the division and the round. An
    /// absent division and an empty one hash the same, so they compare the same here.
    fn same_natural_key(&self, other: &Self) -> bool {
        self.meet == other.meet
            && self.kind == other.kind
            && self.gender == other.gender
            && self.division.as_deref().unwrap_or("") == other.division.as_deref().unwrap_or("")
            && self.round.as_deref().unwrap_or("") == other.round.as_deref().unwrap_or("")
    }

    fn natural_key(&self) -> String {
        format!(
            "event {:?} of {} ({:?} division {:?} round {:?})",
            self.kind,
            self.meet,
            self.gender,
            self.division.as_deref().unwrap_or(""),
            self.round.as_deref().unwrap_or("")
        )
    }

    fn sources(&self) -> String {
        evidence_list(&self.evidence)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalPerformance {
    /// `PerformanceId::mint` hashes the athlete, the meet, the event kind, the date and the provider's
    /// own result key. A row does not retain the kind — the event id already binds it, so two rows
    /// differing only in kind never share an id to begin with — and this compares the four parts a
    /// stored row can restate.
    fn same_natural_key(&self, other: &Self) -> bool {
        self.athlete == other.athlete
            && self.meet == other.meet
            && self.date == other.date
            && self.source_key == other.source_key
    }

    fn natural_key(&self) -> String {
        format!(
            "performance of {} at {} on {} under source key {:?}",
            self.athlete, self.meet, self.date, self.source_key
        )
    }

    fn sources(&self) -> String {
        evidence_list(&self.evidence)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}
