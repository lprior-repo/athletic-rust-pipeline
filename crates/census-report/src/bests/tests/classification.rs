use super::*;

#[test]
fn surface_indoor_only() {
    let meet = test_meet(vec![Sport::IndoorTrack]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Indoor
    );
}

#[test]
fn surface_outdoor_only() {
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Outdoor
    );
}

#[test]
fn surface_xc_only() {
    let meet = test_meet(vec![Sport::CrossCountry]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::CrossCountry
    );
}

#[test]
fn surface_ambiguous_excluded() {
    let meet = test_meet(vec![Sport::IndoorTrack, Sport::OutdoorTrack]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Unresolved
    );
}

#[test]
fn surface_empty_excluded() {
    let meet = test_meet(vec![]);
    assert_eq!(
        crate::bests::key::SurfaceClass::resolve(&meet),
        crate::bests::key::SurfaceClass::Unresolved
    );
}

#[test]
fn wind_sensitive_sprints_hurdles() {
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::Track100m));
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::Track200m));
    assert!(crate::bests::key::is_wind_sensitive(
        &EventKind::Track100mHurdles
    ));
    assert!(crate::bests::key::is_wind_sensitive(
        &EventKind::Track110mHurdles
    ));
}

#[test]
fn wind_sensitive_jump_events() {
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::LongJump));
    assert!(crate::bests::key::is_wind_sensitive(&EventKind::TripleJump));
}

#[test]
fn wind_not_sensitive_400m_hurdles() {
    assert!(!crate::bests::key::is_wind_sensitive(&EventKind::Track400m));
    assert!(!crate::bests::key::is_wind_sensitive(
        &EventKind::Track300mHurdles
    ));
    assert!(!crate::bests::key::is_wind_sensitive(
        &EventKind::Track400mHurdles
    ));
}

#[test]
fn wind_not_sensitive_field_xc() {
    assert!(!crate::bests::key::is_wind_sensitive(&EventKind::HighJump));
    assert!(!crate::bests::key::is_wind_sensitive(&EventKind::ShotPut));
    assert!(!crate::bests::key::is_wind_sensitive(
        &EventKind::CrossCountry
    ));
}

#[test]
fn wind_legal_at_exactly_2_0() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            Some(2.0),
        ),
        crate::bests::key::WindClass::Legal,
    );
}

#[test]
fn wind_legal_below_2_0() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            Some(0.5),
        ),
        crate::bests::key::WindClass::Legal,
    );
}

#[test]
fn wind_assisted_above_2_0() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            Some(2.1),
        ),
        crate::bests::key::WindClass::Assisted,
    );
}

#[test]
fn wind_unknown_missing_wind() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track100m,
            None,
        ),
        crate::bests::key::WindClass::Unknown,
    );
}

#[test]
fn non_finite_wind_cannot_establish_legal_or_assisted_results() {
    for wind in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            crate::bests::classify_wind(
                crate::bests::SurfaceClass::Outdoor,
                &EventKind::Track100m,
                Some(wind)
            ),
            crate::bests::WindClass::Unknown
        );
    }
}

#[test]
fn wind_not_applicable_indoor() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Indoor,
            &EventKind::Track100m,
            Some(3.0),
        ),
        crate::bests::key::WindClass::NotApplicable,
    );
}

#[test]
fn wind_not_applicable_non_sensitive_event() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::Outdoor,
            &EventKind::Track400m,
            Some(3.0),
        ),
        crate::bests::key::WindClass::NotApplicable,
    );
}

#[test]
fn wind_not_applicable_xc() {
    assert_eq!(
        crate::bests::key::classify_wind(
            crate::bests::key::SurfaceClass::CrossCountry,
            &EventKind::CrossCountry,
            Some(5.0),
        ),
        crate::bests::key::WindClass::NotApplicable,
    );
}

#[test]
fn timing_fat() -> TestResult {
    let mark = Mark::TimeSeconds(ExactSeconds::parse("10.94")?);
    check!(
        eq;
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Fat)),
        crate::bests::key::TimingClass::Fat
    );
    Ok(())
}

#[test]
fn timing_hand() -> TestResult {
    let mark = Mark::TimeSeconds(ExactSeconds::parse("10.94")?);
    check!(
        eq;
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Hand)),
        crate::bests::key::TimingClass::Hand
    );
    Ok(())
}

#[test]
fn timing_unknown_explicit() -> TestResult {
    let mark = Mark::TimeSeconds(ExactSeconds::parse("10.94")?);
    check!(
        eq;
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Unknown)),
        crate::bests::key::TimingClass::Unknown
    );
    Ok(())
}

#[test]
fn timing_none_with_time_mark_is_unknown() -> TestResult {
    let mark = Mark::TimeSeconds(ExactSeconds::parse("10.94")?);
    check!(
        eq;
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::Unknown
    );
    Ok(())
}

#[test]
fn timing_none_with_distance_mark_is_non_time() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::NonTime,
    );
}

#[test]
fn timing_none_with_points_mark_is_non_time() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::NonTime,
    );
}

#[test]
fn timing_none_with_raw_mark_is_non_time() {
    let mark = Mark::Raw("unparsed".to_string());
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, None),
        crate::bests::key::TimingClass::NonTime,
    );
}

#[test]
fn timing_annotations_do_not_classify_field_marks() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(
        crate::bests::key::resolve_timing(&mark, Some(TimingMethod::Hand)),
        crate::bests::key::TimingClass::NonTime,
    );
}
