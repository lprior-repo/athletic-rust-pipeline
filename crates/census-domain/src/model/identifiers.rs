use super::*;

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
    /// The candidate role of [`Athlete`]: what one source's observation mints.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct AthleteCandidate;
    /// A search bucket, never a source subject or accepted person.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct AthleteIndex;
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
    pub fn cast<U>(&self) -> Id<U> {
        Id {
            value: self.value.clone(),
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
pub type AthleteIndexId = Id<tag::AthleteIndex>;
/// A canonical athlete *candidate*'s id: what one source's observation of a (school, name, class,
/// gender side) mints.
///
/// Distinct from [`AthleteId`], the resolved cluster's id, even though both print the same value for
/// a cluster of one candidate — which is every row until a decision resolves two candidates into one
/// cluster. Keeping the two roles in two types is what stops a departed candidate's id from being
/// read as the cluster's.
pub type AthleteCandidateId = Id<tag::AthleteCandidate>;
pub type MeetId = Id<tag::Meet>;
pub type EventId = Id<tag::Event>;
pub type PerformanceId = Id<tag::Performance>;
