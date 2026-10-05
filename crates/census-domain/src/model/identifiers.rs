use super::*;

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
    pub struct AthleteCandidate;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct AthleteIndex;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Meet;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Event;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Performance;
}

pub trait IdTag {
    const PREFIX: &'static str;
}

impl IdTag for tag::School {
    const PREFIX: &'static str = "sch";
}

impl IdTag for tag::Team {
    const PREFIX: &'static str = "team";
}

impl IdTag for tag::Coach {
    const PREFIX: &'static str = "coach";
}

impl IdTag for tag::Athlete {
    const PREFIX: &'static str = "ath_subject";
}

impl IdTag for tag::AthleteCandidate {
    const PREFIX: &'static str = "ath";
}

impl IdTag for tag::AthleteIndex {
    const PREFIX: &'static str = "ath";
}

impl IdTag for tag::Meet {
    const PREFIX: &'static str = "meet";
}

impl IdTag for tag::Event {
    const PREFIX: &'static str = "evt";
}

impl IdTag for tag::Performance {
    const PREFIX: &'static str = "perf";
}

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
        write_escaped(prefix.as_bytes(), |chunk| hasher.update(chunk));
        for part in parts {
            hasher.update([0x1f]);
            write_escaped(part.as_bytes(), |chunk| hasher.update(chunk));
        }
        let digest = hasher.finalize();
        let mut hex = String::with_capacity(17 + 16);
        hex.push_str(prefix);
        hex.push('_');
        for byte in digest.iter().take(8) {
            hex.push_str(&format!("{byte:02x}"));
        }
        Self {
            value: hex,
            _tag: PhantomData,
        }
    }

    pub fn validate(&self, expected_prefix: &str) -> bool {
        self.value.starts_with(&format!("{expected_prefix}_"))
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl<T: IdTag> Id<T> {
    pub fn cast<U: IdTag>(&self) -> Id<U> {
        let prefix = T::PREFIX;
        if !self.validate(prefix) {
            debug_assert!(
                false,
                "Id value {self} does not match expected prefix {prefix}"
            );
        }
        Id {
            value: self.value.clone(),
            _tag: PhantomData,
        }
    }
}

impl<T> std::borrow::Borrow<str> for Id<T> {
    fn borrow(&self) -> &str {
        self.as_str()
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
pub type AthleteCandidateId = Id<tag::AthleteCandidate>;
pub type MeetId = Id<tag::Meet>;
pub type EventId = Id<tag::Event>;
pub type PerformanceId = Id<tag::Performance>;

const FRAMING_DELIMITERS: [u8; 3] = [0x1d, 0x1e, 0x1f];

pub(crate) fn write_escaped<W: FnMut(&[u8])>(bytes: &[u8], mut write: W) {
    if !bytes.iter().any(|byte| FRAMING_DELIMITERS.contains(byte)) {
        write(bytes);
        return;
    }
    for byte in bytes {
        match byte {
            0x1d => write(&[0x1d, 0x00]),
            0x1e => write(&[0x1d, 0x01]),
            0x1f => write(&[0x1d, 0x02]),
            other => write(&[*other]),
        }
    }
}
