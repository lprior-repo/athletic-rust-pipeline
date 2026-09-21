//! Event vocabulary: the normalized event name, its value-kind classification,
//! and the key (`parse`) and display-name (`name`) tables it is built from. The
//! wire format lives in `wire`.

use crate::domain::error::DomainError;

mod name;
mod parse;
mod wire;

use self::name::{cross_country_name, relay_name, track_name};
use self::parse::{event_key, field_event, hurdles_event, relay_event, track_event};

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
