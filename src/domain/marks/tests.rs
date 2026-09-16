use super::*;
use crate::domain::error::DomainError;

fn event(raw: &str) -> EventName {
    EventName::parse(raw).expect("valid fixture event")
}

fn performance(event: &EventName, raw: &str) -> Performance {
    Performance::parse(event, raw).expect("valid fixture mark")
}

#[test]
fn parses_time_to_integral_milliseconds() {
    let item = performance(&event("100m"), "10.55s");
    assert_eq!(
        item.comparable(),
        Some(MarkValue::Time(Milliseconds(10_550)))
    );
    let split = performance(&event("800m"), "1:02.500");
    assert_eq!(
        split.comparable(),
        Some(MarkValue::Time(Milliseconds(62_500)))
    );
}

#[test]
fn converts_metric_distance_and_exact_imperial() {
    let metric = performance(&event("long jump"), "6.25m");
    assert_eq!(
        metric.comparable(),
        Some(MarkValue::Distance(Nanometers(6_250_000_000)))
    );
    let imperial = performance(&event("high jump"), "6'2\"");
    assert_eq!(
        imperial.comparable(),
        Some(MarkValue::Distance(Nanometers(1_879_600_000)))
    );
}

#[test]
fn rejects_negative_nonfinite_and_malformed_marks() {
    let event = event("100m");
    assert_eq!(
        Performance::parse(&event, "-1.2"),
        Err(DomainError::OutOfRange { field: "mark" })
    );
    assert_eq!(
        Performance::parse(&event, "NaN"),
        Err(DomainError::InvalidFormat { field: "mark" })
    );
    assert_eq!(
        Performance::parse(&event, "not-a-mark"),
        Err(DomainError::InvalidFormat { field: "mark" })
    );
}

#[test]
fn compares_directionally_and_does_not_cross_events() {
    let sprint = event("100m");
    let jump = event("long jump");
    assert_eq!(
        compare_performances(&performance(&sprint, "10"), &performance(&sprint, "11")),
        Comparison::Better
    );
    assert_eq!(
        compare_performances(&performance(&jump, "7m"), &performance(&jump, "6m")),
        Comparison::Better
    );
    assert_eq!(
        compare_performances(&performance(&sprint, "10"), &performance(&jump, "10m")),
        Comparison::IncompatibleEvent
    );
}

#[test]
fn nonfinish_states_are_retained_and_not_comparable() {
    let event = event("400m");
    let dns = performance(&event, "DNS");
    let dnf = performance(&event, "DNF");
    let dq = performance(&event, "DQ");
    let foul = performance(&self::event("long jump"), "foul");
    assert_eq!(dns.state(), &PerformanceState::Dns);
    assert_eq!(dnf.state(), &PerformanceState::Dnf);
    assert_eq!(dq.state(), &PerformanceState::Dq);
    assert_eq!(foul.state(), &PerformanceState::Foul);
    assert_eq!(performance(&event, "-").state(), &PerformanceState::NoMark);
    assert_eq!(
        compare_performances(&dns, &performance(&event, "50")),
        Comparison::NonComparable
    );
}

#[test]
fn unsupported_event_and_mark_are_retained_for_review() {
    let unknown_event =
        EventName::parse_retained("steeplechase").unwrap_or(EventName::CrossCountryUnknown);
    let item = performance(&unknown_event, "6:00");
    assert_eq!(item.raw(), "6:00");
    assert!(matches!(item.state(), PerformanceState::Unsupported { .. }));
    assert_eq!(unknown_event.as_str(), "steeplechase");
    let sprint = event("100m");
    let malformed = Performance::parse_retained(&sprint, "wind-assisted");
    assert!(matches!(
        malformed,
        Ok(Performance {
            state: PerformanceState::Unsupported { .. },
            ..
        })
    ));
}

#[test]
fn best_mark_filters_to_requested_event() {
    let sprint = event("100m");
    let jump = event("long jump");
    let items = vec![
        performance(&sprint, "11"),
        performance(&jump, "8m"),
        performance(&sprint, "10"),
    ];
    let best = best_performance(&sprint, &items);
    assert_eq!(best.map(Performance::raw), Some("10"));
}

#[test]
fn parses_added_hurdle_and_relay_events() {
    assert_eq!(event("80m").as_str(), "80m");
    assert_eq!(event("500m").as_str(), "500m");
    assert_eq!(event("2000m").as_str(), "2000m");
    assert_eq!(event("60h").as_str(), "60h");
    assert_eq!(event("4x400 relay").as_str(), "4x400");
}

#[test]
fn unitless_distance_is_retained_not_assumed_meters() {
    let jump = event("long jump");
    assert_eq!(
        Performance::parse(&jump, "6.25"),
        Err(DomainError::InvalidFormat { field: "mark" })
    );
    assert!(matches!(
        Performance::parse_retained(&jump, "6.25"),
        Ok(Performance {
            state: PerformanceState::Unsupported { .. },
            ..
        })
    ));
}

#[test]
fn cross_country_distance_and_combined_event_identity_are_preserved() {
    let xc5 = event("xc 5k");
    let xc10 = event("xc 10k");
    assert_eq!(
        compare_performances(&performance(&xc5, "20:00"), &performance(&xc10, "40:00")),
        Comparison::IncompatibleEvent
    );
    let unknown = event("cross country");
    assert_eq!(
        compare_performances(
            &performance(&unknown, "20:00"),
            &performance(&unknown, "21:00")
        ),
        Comparison::NonComparable
    );
    assert_ne!(event("decathlon"), event("heptathlon"));
}

#[test]
fn imperial_inches_are_exact_and_excess_precision_is_rejected() {
    let jump = event("long jump");
    assert!(matches!(
        Performance::parse_retained(&jump, "6'12\""),
        Ok(Performance {
            state: PerformanceState::Unsupported { .. },
            ..
        })
    ));
    assert_eq!(
        Performance::parse(&jump, "6'2.0011\""),
        Err(DomainError::InvalidFormat { field: "mark" })
    );
}
