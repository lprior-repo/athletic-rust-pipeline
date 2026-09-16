use crate::domain::error::DomainError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventName {
    Track(TrackEvent),
    Relay(RelayEvent),
    LongJump,
    TripleJump,
    HighJump,
    PoleVault,
    ShotPut,
    Discus,
    Javelin,
    Hammer,
    WeightThrow,
    Decathlon,
    Heptathlon,
    Pentathlon,
    CrossCountry(CrossCountryEvent),
    CrossCountryUnknown,
    Unsupported(String),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackEvent {
    meters: u16,
    hurdles: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelayEvent {
    meters: u16,
    legs: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CrossCountryEvent {
    meters: u16,
}

#[derive(Clone, Copy)]
pub(super) enum ValueKind {
    Time,
    Distance,
    Points,
    Unsupported,
}

impl EventName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let key = event_key(raw)?;
        Self::from_key(&key).ok_or(DomainError::Unsupported { field: "event" })
    }
    pub fn parse_retained(raw: &str) -> Result<Self, DomainError> {
        let key = event_key(raw)?;
        match Self::from_key(&key) {
            Some(event) => Ok(event),
            None => Ok(Self::Unsupported(raw.to_owned())),
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Track(event) => track_name(event.meters, event.hurdles),
            Self::Relay(event) => relay_name(event.meters, event.legs),
            Self::LongJump => "long_jump",
            Self::TripleJump => "triple_jump",
            Self::HighJump => "high_jump",
            Self::PoleVault => "pole_vault",
            Self::ShotPut => "shot_put",
            Self::Discus => "discus",
            Self::Javelin => "javelin",
            Self::Hammer => "hammer",
            Self::WeightThrow => "weight_throw",
            Self::Decathlon => "decathlon",
            Self::Heptathlon => "heptathlon",
            Self::Pentathlon => "pentathlon",
            Self::CrossCountry(event) => cross_country_name(event.meters),
            Self::CrossCountryUnknown => "cross_country",
            Self::Unsupported(raw) => raw,
        }
    }
    pub fn is_lower_better(&self) -> bool {
        !matches!(
            self,
            Self::LongJump
                | Self::TripleJump
                | Self::HighJump
                | Self::PoleVault
                | Self::ShotPut
                | Self::Discus
                | Self::Javelin
                | Self::Hammer
                | Self::WeightThrow
                | Self::Decathlon
                | Self::Heptathlon
                | Self::Pentathlon
                | Self::Unsupported(_)
        )
    }
    pub(super) fn is_comparable_event(&self) -> bool {
        !matches!(self, Self::CrossCountryUnknown | Self::Unsupported(_))
    }
    fn from_key(key: &str) -> Option<Self> {
        let key = match key
            .strip_suffix("meters")
            .or_else(|| key.strip_suffix("meter"))
        {
            Some(base) => base,
            None => key,
        };
        track_event(key)
            .or_else(|| hurdles_event(key))
            .or_else(|| relay_event(key))
            .or_else(|| field_event(key))
    }
    pub(super) fn value_kind(&self) -> ValueKind {
        match self {
            Self::Track(_) | Self::Relay(_) | Self::CrossCountry(_) => ValueKind::Time,
            Self::CrossCountryUnknown | Self::Unsupported(_) => ValueKind::Unsupported,
            Self::LongJump
            | Self::TripleJump
            | Self::HighJump
            | Self::PoleVault
            | Self::ShotPut
            | Self::Discus
            | Self::Javelin
            | Self::Hammer
            | Self::WeightThrow => ValueKind::Distance,
            Self::Decathlon | Self::Heptathlon | Self::Pentathlon => ValueKind::Points,
        }
    }
}

fn event_key(raw: &str) -> Result<String, DomainError> {
    if raw.len() > 80 {
        return Err(DomainError::TooLong {
            field: "event",
            limit: 80,
        });
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Empty { field: "event" });
    }
    let numeric_hyphen = trimmed.as_bytes().windows(3).any(|window| {
        matches!(window, [left, b'-', right] if left.is_ascii_digit() && right.is_ascii_digit())
    });
    Ok(trimmed
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_' && (*ch != '-' || numeric_hyphen))
        .flat_map(char::to_lowercase)
        .collect())
}
fn track_event(key: &str) -> Option<EventName> {
    [
        55, 60, 80, 100, 200, 300, 400, 500, 600, 800, 1000, 1500, 1600, 2000, 3000, 3200, 5000,
        10000,
    ]
    .iter()
    .find_map(|meters| {
        let number = meters.to_string();
        (key == number || key == number + "m").then_some(EventName::Track(TrackEvent {
            meters: *meters,
            hurdles: false,
        }))
    })
    .or(match key {
        "5k" | "5km" => Some(EventName::Track(TrackEvent {
            meters: 5000,
            hurdles: false,
        })),
        "10k" | "10km" => Some(EventName::Track(TrackEvent {
            meters: 10000,
            hurdles: false,
        })),
        "mile" => Some(EventName::Track(TrackEvent {
            meters: 1609,
            hurdles: false,
        })),
        _ => None,
    })
}
fn field_event(key: &str) -> Option<EventName> {
    match key {
        "longjump" | "lj" => Some(EventName::LongJump),
        "triplejump" | "tj" => Some(EventName::TripleJump),
        "highjump" | "hj" => Some(EventName::HighJump),
        "polevault" | "pv" => Some(EventName::PoleVault),
        "shotput" => Some(EventName::ShotPut),
        "discus" => Some(EventName::Discus),
        "javelin" => Some(EventName::Javelin),
        "hammer" => Some(EventName::Hammer),
        "weightthrow" => Some(EventName::WeightThrow),
        "decathlon" => Some(EventName::Decathlon),
        "heptathlon" => Some(EventName::Heptathlon),
        "pentathlon" => Some(EventName::Pentathlon),
        "xc" | "crosscountry" => Some(EventName::CrossCountryUnknown),
        "xc2k" | "crosscountry2k" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 2_000 }))
        }
        "xc3k" | "crosscountry3k" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 3_000 }))
        }
        "xc4k" | "crosscountry4k" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 4_000 }))
        }
        "xc5k" | "crosscountry5k" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 5_000 }))
        }
        "xc6k" | "crosscountry6k" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 6_000 }))
        }
        "xc8k" | "crosscountry8k" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 8_000 }))
        }
        "xc10k" | "crosscountry10k" => Some(EventName::CrossCountry(CrossCountryEvent {
            meters: 10_000,
        })),
        "xc12k" | "crosscountry12k" => Some(EventName::CrossCountry(CrossCountryEvent {
            meters: 12_000,
        })),
        "xc2mile" | "crosscountry2mile" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 3_219 }))
        }
        "xc3mile" | "crosscountry3mile" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 4_828 }))
        }
        "xc5mile" | "crosscountry5mile" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 8_047 }))
        }
        "xc6mile" | "crosscountry6mile" => {
            Some(EventName::CrossCountry(CrossCountryEvent { meters: 9_656 }))
        }
        _ => None,
    }
}
fn hurdles_event(key: &str) -> Option<EventName> {
    let distance = [
        "mhurdles",
        "meterhurdles",
        "metershurdles",
        "hurdles",
        "mh",
        "h",
    ]
    .into_iter()
    .find_map(|suffix| key.strip_suffix(suffix))?;
    let meters = distance.parse::<u16>().ok()?;
    [55, 60, 80, 100, 110, 300, 400]
        .contains(&meters)
        .then_some(EventName::Track(TrackEvent {
            meters,
            hurdles: true,
        }))
}
fn relay_event(key: &str) -> Option<EventName> {
    [(100, 4), (200, 4), (400, 4), (800, 4), (1600, 4)]
        .iter()
        .find_map(|(meters, legs)| {
            let expected = format!("{legs}x{meters}");
            let expected_m = format!("{expected}m");
            let relay = format!("{expected}relay");
            let m_relay = format!("{expected_m}relay");
            (key == expected || key == expected_m || key == relay || key == m_relay).then_some(
                EventName::Relay(RelayEvent {
                    meters: *meters,
                    legs: *legs,
                }),
            )
        })
}
fn track_name(meters: u16, hurdles: bool) -> &'static str {
    match hurdles {
        true => hurdle_name(meters),
        false => flat_name(meters),
    }
}
fn flat_name(meters: u16) -> &'static str {
    match meters {
        55 => "55m",
        60 => "60m",
        80 => "80m",
        100 => "100m",
        200 => "200m",
        300 => "300m",
        400 => "400m",
        500 => "500m",
        600 => "600m",
        800 => "800m",
        1000 => "1000m",
        1500 => "1500m",
        1600 => "1600m",
        2000 => "2000m",
        3000 => "3000m",
        3200 => "3200m",
        5000 => "5000m",
        10000 => "10000m",
        1609 => "mile",
        _ => "unsupported",
    }
}
fn hurdle_name(meters: u16) -> &'static str {
    match meters {
        55 => "55h",
        60 => "60h",
        80 => "80h",
        100 => "100h",
        110 => "110h",
        300 => "300h",
        400 => "400h",
        _ => "unsupported",
    }
}
fn relay_name(meters: u16, legs: u8) -> &'static str {
    match (meters, legs) {
        (100, 4) => "4x100",
        (200, 4) => "4x200",
        (400, 4) => "4x400",
        (800, 4) => "4x800",
        (1600, 4) => "4x1600",
        _ => "unsupported",
    }
}
fn cross_country_name(meters: u16) -> &'static str {
    match meters {
        2_000 => "xc2k",
        3_000 => "xc3k",
        4_000 => "xc4k",
        5_000 => "xc5k",
        6_000 => "xc6k",
        8_000 => "xc8k",
        10_000 => "xc10k",
        12_000 => "xc12k",
        3_219 => "xc2mile",
        4_828 => "xc3mile",
        8_047 => "xc5mile",
        9_656 => "xc6mile",
        _ => "cross_country",
    }
}
impl Serialize for EventName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for EventName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse_retained(&raw).map_err(serde::de::Error::custom)
    }
}
