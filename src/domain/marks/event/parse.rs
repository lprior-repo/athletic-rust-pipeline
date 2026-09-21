//! Raw source event keys to `EventName`: key normalization plus the track,
//! hurdle, relay and field tables.

use super::{CrossCountryEvent, EventName, RelayEvent, TrackEvent};
use crate::domain::error::DomainError;

pub(super) fn event_key(raw: &str) -> Result<String, DomainError> {
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
pub(super) fn track_event(key: &str) -> Option<EventName> {
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
pub(super) fn field_event(key: &str) -> Option<EventName> {
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
pub(super) fn hurdles_event(key: &str) -> Option<EventName> {
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
pub(super) fn relay_event(key: &str) -> Option<EventName> {
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
