use super::*;

mod qualified;

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
        match self {
            Self::Track100m => Cow::Borrowed("Track100m"),
            Self::Track200m => Cow::Borrowed("Track200m"),
            Self::Track400m => Cow::Borrowed("Track400m"),
            Self::Track800m => Cow::Borrowed("Track800m"),
            Self::Track1600m => Cow::Borrowed("Track1600m"),
            Self::Track3200m => Cow::Borrowed("Track3200m"),
            Self::Track1Mile => Cow::Borrowed("Track1Mile"),
            Self::Track3000m => Cow::Borrowed("Track3000m"),
            Self::Track5000m => Cow::Borrowed("Track5000m"),
            Self::Track110mHurdles => Cow::Borrowed("Track110mHurdles"),
            Self::Track100mHurdles => Cow::Borrowed("Track100mHurdles"),
            Self::Track300mHurdles => Cow::Borrowed("Track300mHurdles"),
            Self::Track400mHurdles => Cow::Borrowed("Track400mHurdles"),
            Self::Track2000mSteeplechase => Cow::Borrowed("Track2000mSteeplechase"),
            Self::Track3000mSteeplechase => Cow::Borrowed("Track3000mSteeplechase"),
            Self::CrossCountry => Cow::Borrowed("CrossCountry"),
            Self::Relay4x100 => Cow::Borrowed("Relay4x100"),
            Self::Relay4x200 => Cow::Borrowed("Relay4x200"),
            Self::Relay4x400 => Cow::Borrowed("Relay4x400"),
            Self::Relay4x800 => Cow::Borrowed("Relay4x800"),
            Self::SprintMedley => Cow::Borrowed("SprintMedley"),
            Self::DistanceMedley => Cow::Borrowed("DistanceMedley"),
            Self::HighJump => Cow::Borrowed("HighJump"),
            Self::LongJump => Cow::Borrowed("LongJump"),
            Self::TripleJump => Cow::Borrowed("TripleJump"),
            Self::PoleVault => Cow::Borrowed("PoleVault"),
            Self::ShotPut => Cow::Borrowed("ShotPut"),
            Self::Discus => Cow::Borrowed("Discus"),
            Self::Javelin => Cow::Borrowed("Javelin"),
            Self::Hammer => Cow::Borrowed("Hammer"),
            Self::WeightThrow => Cow::Borrowed("WeightThrow"),
            Self::Pentathlon => Cow::Borrowed("Pentathlon"),
            Self::Heptathlon => Cow::Borrowed("Heptathlon"),
            Self::Decathlon => Cow::Borrowed("Decathlon"),
            Self::Unmapped { label } => Cow::Owned(format!("Unmapped {{ label: {label:?} }}")),
        }
    }

    pub fn from_source_label(label: &str) -> Self {
        let direct = Self::direct_label(label);
        if !matches!(direct, EventKind::Unmapped { .. }) {
            return direct;
        }
        for candidate in qualified::candidates(label) {
            let kind = Self::direct_label(&candidate);
            if !matches!(kind, EventKind::Unmapped { .. }) {
                return kind;
            }
        }
        EventKind::Unmapped {
            label: label.trim().to_string(),
        }
    }

    fn direct_label(label: &str) -> Self {
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
            "300mh" | "300h" | "300mhurdles" | "300mhhurdles" | "300hurdles" => {
                EventKind::Track300mHurdles
            }
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

    fn normalized_label(label: &str) -> String {
        let normalized: String = label
            .chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '_' | ',' | '\'' | '’'))
            .collect::<String>()
            .to_ascii_lowercase();
        normalized
            .replace("meters", "m")
            .replace("metre", "m")
            .replace("meter", "m")
    }

    fn division_code(token: &str) -> bool {
        fn code(part: &str) -> bool {
            match part.as_bytes() {
                [first, second] => {
                    (first.is_ascii_digit() && matches!(second, b'a'..=b'f'))
                        || (*first == b'd' && second.is_ascii_digit())
                }
                _ => false,
            }
        }
        let lower = token.to_ascii_lowercase();
        match lower.split_once('-') {
            Some((head, tail)) => code(head) && code(tail),
            None => code(&lower),
        }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEventLabel {
    pub source: SourceRef,
    pub label: String,
}
