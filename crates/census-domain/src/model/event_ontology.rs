use super::*;

mod direct;
mod qualified;
mod stable;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Track55m,
    Track60m,
    Track1000m,
    Track1500m,
    Track55mHurdles,
    Track60mHurdles,
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
    Unmapped { label: String },
}

const LABEL_QUALIFIERS: &[&str] = &[
    "girls",
    "boys",
    "womens",
    "women",
    "mens",
    "men",
    "male",
    "female",
    "mixed",
    "coed",
    "results",
    "finals",
    "final",
    "prelims",
    "prelim",
    "preliminary",
    "preliminaries",
    "semifinals",
    "semifinal",
    "semis",
    "heats",
    "varsity",
    "jv",
    "junior",
    "senior",
    "freshman",
    "freshmen",
    "frosh",
    "sophomore",
    "sophomores",
    "open",
    "championship",
    "championships",
    "champ",
    "division",
    "div",
    "class",
    "invitational",
    "dash",
    "run",
];

const LABEL_PHRASES: &[(&str, &str)] = &[("high", "school")];

impl EventKind {
    pub fn stable_key(&self) -> Cow<'_, str> {
        if let Self::Unmapped { label } = self {
            return Cow::Owned(format!("Unmapped {{ label: {label:?} }}"));
        }
        match stable::track(self)
            .or_else(|| stable::hurdles(self))
            .or_else(|| stable::other(self))
        {
            Some(key) => Cow::Borrowed(key),
            None => Cow::Owned(format!("{self:?}")),
        }
    }

    pub fn from_source_label(label: &str) -> Self {
        if let Some(kind) = direct::resolve(&Self::normalized_label(label)) {
            return kind;
        }
        match qualified::candidates(label)
            .into_iter()
            .find_map(|candidate| direct::resolve(&Self::normalized_label(&candidate)))
        {
            Some(kind) => kind,
            None => Self::Unmapped {
                label: label.trim().to_string(),
            },
        }
    }

    fn normalized_label(label: &str) -> String {
        let normalized: String = label
            .chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '_' | ',' | '\'' | '’'))
            .collect::<String>()
            .to_ascii_lowercase();
        normalized
            .replace("meters", "m")
            .replace("metres", "m")
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

    pub fn is_hurdles(&self) -> bool {
        matches!(
            self,
            Self::Track55mHurdles
                | Self::Track60mHurdles
                | Self::Track100mHurdles
                | Self::Track110mHurdles
                | Self::Track300mHurdles
                | Self::Track400mHurdles
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEventLabel {
    pub source: SourceRef,
    pub label: String,
}
