use crate::domain::error::DomainError;
use serde::{Deserialize, Serialize};

mod compare;
mod event;
mod parser;

use compare::compare_values;
use event::ValueKind;
pub use event::{CrossCountryEvent, EventName, RelayEvent, TrackEvent};
use parser::{parse_distance, parse_integer, parse_time};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Milliseconds(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nanometers(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Points(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Count(u64);

macro_rules! scalar_serde {
    ($name:ident) => {
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_u64(self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = u64::deserialize(deserializer)?;
                (value > 0).then_some(Self(value)).ok_or_else(|| {
                    serde::de::Error::custom("validated mark values must be positive")
                })
            }
        }
    };
}

scalar_serde!(Milliseconds);
scalar_serde!(Nanometers);
scalar_serde!(Points);
scalar_serde!(Count);

impl Milliseconds {
    pub fn get(self) -> u64 {
        self.0
    }
}

impl Nanometers {
    pub fn get(self) -> u64 {
        self.0
    }
}

impl Points {
    pub fn get(self) -> u64 {
        self.0
    }
}

impl Count {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MarkValue {
    Time(Milliseconds),
    Distance(Nanometers),
    Points(Points),
    Count(Count),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerformanceState {
    Recorded(MarkValue),
    Dns,
    Dnf,
    Dq,
    Foul,
    NoMark,
    Unsupported { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Performance {
    event: EventName,
    raw: String,
    state: PerformanceState,
}

impl<'de> Deserialize<'de> for Performance {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Boundary {
            event: EventName,
            raw: String,
            state: PerformanceState,
        }
        let input = Boundary::deserialize(deserializer)?;
        let parsed =
            Self::parse_retained(&input.event, &input.raw).map_err(serde::de::Error::custom)?;
        let consistent = match (&input.state, &parsed.state) {
            (PerformanceState::Unsupported { .. }, PerformanceState::Unsupported { .. }) => true,
            (supplied, expected) => supplied == expected,
        };
        if !consistent {
            return Err(serde::de::Error::custom(
                "performance state contradicts its event or raw mark",
            ));
        }
        Ok(parsed)
    }
}

impl Performance {
    pub(crate) fn from_status(event: &EventName, raw: &str) -> Option<Self> {
        if raw.len() > 120 {
            return None;
        }
        status_state(raw).map(|state| Self {
            event: event.clone(),
            raw: raw.to_owned(),
            state,
        })
    }
    pub fn parse(event: &EventName, raw: &str) -> Result<Self, DomainError> {
        if raw.len() > 120 {
            return Err(DomainError::TooLong {
                field: "mark",
                limit: 120,
            });
        }
        let value = raw.trim();
        if value.is_empty() {
            return Err(DomainError::Empty { field: "mark" });
        }
        let state = match status_state(value) {
            Some(state) => state,
            None => parse_value(event, value)?,
        };
        Ok(Self {
            event: event.clone(),
            raw: raw.to_owned(),
            state,
        })
    }

    pub fn event(&self) -> &EventName {
        &self.event
    }
    pub fn raw(&self) -> &str {
        &self.raw
    }
    pub fn state(&self) -> &PerformanceState {
        &self.state
    }

    pub fn comparable(&self) -> Option<MarkValue> {
        match self.state {
            PerformanceState::Recorded(value) => Some(value),
            PerformanceState::Dns
            | PerformanceState::Dnf
            | PerformanceState::Dq
            | PerformanceState::Foul
            | PerformanceState::NoMark
            | PerformanceState::Unsupported { .. } => None,
        }
    }

    pub fn is_recorded(&self) -> bool {
        matches!(self.state, PerformanceState::Recorded(_))
    }

    pub fn parse_retained(event: &EventName, raw: &str) -> Result<Self, DomainError> {
        match Self::parse(event, raw) {
            Ok(value) => Ok(value),
            Err(
                reason @ (DomainError::InvalidFormat { .. }
                | DomainError::Unsupported { .. }
                | DomainError::OutOfRange { .. }),
            ) => Ok(Self {
                event: event.clone(),
                raw: raw.to_owned(),
                state: PerformanceState::Unsupported {
                    reason: reason.to_string(),
                },
            }),
            Err(reason) => Err(reason),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison {
    Better,
    Worse,
    Equal,
    IncompatibleEvent,
    NonComparable,
}

pub fn compare_performances(candidate: &Performance, previous: &Performance) -> Comparison {
    if candidate.event != previous.event {
        return Comparison::IncompatibleEvent;
    }
    if !candidate.event.is_comparable_event() {
        return Comparison::NonComparable;
    }
    let (Some(left), Some(right)) = (candidate.comparable(), previous.comparable()) else {
        return Comparison::NonComparable;
    };
    compare_values(left, right, candidate.event.is_lower_better())
}

pub fn best_performance<'a>(
    event: &EventName,
    performances: &'a [Performance],
) -> Option<&'a Performance> {
    performances
        .iter()
        .filter(|item| item.event == *event && item.is_recorded())
        .fold(None, |best, item| {
            best.filter(|current| compare_performances(item, current) != Comparison::Better)
                .or(Some(item))
        })
}

fn parse_value(event: &EventName, raw: &str) -> Result<PerformanceState, DomainError> {
    match event.value_kind() {
        ValueKind::Time => parse_time(raw)
            .map(MarkValue::Time)
            .map(PerformanceState::Recorded),
        ValueKind::Distance => parse_distance(raw)
            .map(MarkValue::Distance)
            .map(PerformanceState::Recorded),
        ValueKind::Points => parse_integer(raw, "points")
            .and_then(|value| {
                (value > 0)
                    .then_some(value)
                    .ok_or(DomainError::OutOfRange { field: "points" })
            })
            .map(Points)
            .map(MarkValue::Points)
            .map(PerformanceState::Recorded),
        ValueKind::Unsupported => Ok(PerformanceState::Unsupported {
            reason: DomainError::Unsupported { field: "event" }.to_string(),
        }),
    }
}

fn status_state(raw: &str) -> Option<PerformanceState> {
    let trimmed = raw.trim();
    if trimmed == "-" || trimmed == "—" {
        return Some(PerformanceState::NoMark);
    }
    match raw
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
        .as_str()
    {
        "dns" => Some(PerformanceState::Dns),
        "dnf" => Some(PerformanceState::Dnf),
        "dq" | "dsq" => Some(PerformanceState::Dq),
        "foul" | "x" => Some(PerformanceState::Foul),
        "nm" | "nd" | "nh" | "nt" | "nomark" => Some(PerformanceState::NoMark),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
