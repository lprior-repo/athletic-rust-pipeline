use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn identical_observed_event_specification_is_admitted() -> TestResult {
    for label in ["Boys 400 Meter Dash", "Boys 400 Meter Dash (200m banked)"] {
        let kind = EventKind::from_source_label(label);
        check!(eq; kind, EventKind::Track400m);
        require_matching_event(&kind, label, label)?;
    }
    Ok(())
}

#[test]
fn same_distance_and_category_cannot_exchange_plain_and_banked_rows() -> TestResult {
    let plain = "Boys 400 Meter Dash";
    let banked = "Boys 400 Meter Dash (200m banked)";
    let kind = EventKind::Track400m;
    check!(eq; EventKind::from_source_label(plain), kind);
    check!(eq; EventKind::from_source_label(banked), kind);
    let plain_spec = EventSpecification::from_published_label(plain, &kind)?;
    let banked_spec = EventSpecification::from_published_label(banked, &kind)?;
    check!(eq; plain_spec.category, banked_spec.category);
    check!(ne; plain_spec.indoor_track, banked_spec.indoor_track);
    for (original, linked) in [(plain, banked), (banked, plain)] {
        let error = require_matching_event(&kind, original, linked)
            .err()
            .ok_or("different source event specification was admitted")?;
        check!(error
            .to_string()
            .contains("checked linked event contradicts original event"));
    }
    Ok(())
}
