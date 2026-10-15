use super::*;
use crate::model::{
    ContractRefV1, DiscoveryPriorityV1, EventIdentity, EventKind, EventSpecification, Evidence,
    Gender, GradYear, Id, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1, ProfilePoliciesV1,
    ProvisionalInclusionV1, QueryProfileInputV1, RegionalMembershipV1, SchoolYear,
    SourceEventLabel, SourceRef,
};
use crate::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn profile(sports: Vec<Sport>, categories: Vec<Gender>) -> TestResult<QueryProfileV1> {
    let contract = ContractRefV1::new(1, 1, &"a".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports,
        competition_categories: categories,
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: contract.clone(),
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-01")?,
        )?,
        completion_contract: contract.clone(),
        policies: ProfilePoliciesV1 {
            cohort: contract.clone(),
            event: contract.clone(),
            identity: contract.clone(),
            affiliation: contract.clone(),
            contact: contract.clone(),
            participation: contract.clone(),
            publication: contract,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

fn event(kind: EventKind, gender: Gender, label: &str) -> TestResult<CanonicalEvent> {
    let meet = Id::mint("meet", &["selected_catalog_fixture"]);
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet,
            kind,
            gender,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    let source = SourceRef::new(
        "selected_catalog_fixture",
        Some("https://example.invalid/selected-results".into()),
    );
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    event.evidence.push(Evidence::parsed(source, "2026-05-01"));
    Ok(event)
}

#[test]
fn selected_sports_and_categories_retain_each_exclusion_dimension() -> TestResult {
    let catalog = RecruitingCatalogV1::new();
    let profile = profile(vec![Sport::OutdoorTrack], vec![Gender::Girls])?;
    for (sport, gender, expected) in [
        (Sport::OutdoorTrack, Gender::Girls, None),
        (
            Sport::IndoorTrack,
            Gender::Girls,
            Some(EventExclusionReason::OutsideSelectedSport),
        ),
        (
            Sport::OutdoorTrack,
            Gender::Boys,
            Some(EventExclusionReason::OutsideSelectedCompetitionCategory),
        ),
        (
            Sport::IndoorTrack,
            Gender::Boys,
            Some(EventExclusionReason::OutsideSelectedSportAndCompetitionCategory),
        ),
        (
            Sport::OutdoorTrack,
            Gender::Mixed,
            Some(EventExclusionReason::OutsideSelectedCompetitionCategory),
        ),
    ] {
        let event = event(EventKind::Track100m, gender, "100m")?;
        check!(eq; catalog.classify(&event, sport)?, EventDisposition::Included(EventKind::Track100m));
        let expected = match expected {
            Some(reason) => EventDisposition::Excluded(reason),
            None => EventDisposition::Included(EventKind::Track100m),
        };
        check!(eq; catalog.classify_for_profile(&event, sport, &profile)?, expected);
    }
    Ok(())
}

#[test]
fn selected_categories_use_resolved_published_prefix_and_mixed_is_distinct() -> TestResult {
    let catalog = RecruitingCatalogV1::new();
    for (label, category) in [
        ("Boys U18 100m", Gender::Boys),
        ("Girls Varsity 100m", Gender::Girls),
        ("Mixed 100m", Gender::Mixed),
    ] {
        let event = event(EventKind::Track100m, Gender::Unknown, label)?;
        for selected in [Gender::Boys, Gender::Girls, Gender::Mixed] {
            let profile = profile(vec![Sport::OutdoorTrack], vec![selected])?;
            let expected = if selected == category {
                EventDisposition::Included(EventKind::Track100m)
            } else {
                EventDisposition::Excluded(EventExclusionReason::OutsideSelectedCompetitionCategory)
            };
            check!(eq; catalog.classify_for_profile(&event, Sport::OutdoorTrack, &profile)?, expected);
        }
    }
    Ok(())
}

#[test]
fn unknown_and_contradictory_evidence_never_becomes_outside_profile() -> TestResult {
    let catalog = RecruitingCatalogV1::new();
    let profile = profile(vec![Sport::IndoorTrack], vec![Gender::Girls])?;
    for (gender, label, sport, reason) in [
        (
            Gender::Boys,
            "100m",
            Sport::Unknown,
            EventResolutionReason::UnresolvedSport,
        ),
        (
            Gender::Unknown,
            "100m",
            Sport::OutdoorTrack,
            EventResolutionReason::UnresolvedCategoryOrContext,
        ),
        (
            Gender::Unknown,
            "Varsity 100m",
            Sport::OutdoorTrack,
            EventResolutionReason::UnresolvedCategoryOrContext,
        ),
        (
            Gender::Boys,
            "Girls 100m",
            Sport::OutdoorTrack,
            EventResolutionReason::ContradictoryDeclarations,
        ),
        (
            Gender::Girls,
            "Mixed 100m",
            Sport::OutdoorTrack,
            EventResolutionReason::ContradictoryDeclarations,
        ),
    ] {
        let event = event(EventKind::Track100m, gender, label)?;
        let expected = EventDisposition::Unresolved(reason);
        check!(eq; catalog.classify(&event, sport)?, expected);
        check!(eq; catalog.classify_for_profile(&event, sport, &profile)?, expected);
    }
    Ok(())
}

#[test]
fn selected_cross_country_and_multiple_sport_sets_use_catalog_sport_context() -> TestResult {
    let catalog = RecruitingCatalogV1::new();
    let event = event(EventKind::CrossCountry, Gender::Girls, "Cross Country")?;
    let selected = profile(
        vec![Sport::IndoorTrack, Sport::CrossCountry],
        vec![Gender::Girls],
    )?;
    check!(eq; catalog.classify_for_profile(&event, Sport::CrossCountry, &selected)?, EventDisposition::Included(EventKind::CrossCountry));
    let outside = profile(
        vec![Sport::IndoorTrack, Sport::OutdoorTrack],
        vec![Gender::Girls],
    )?;
    check!(eq; catalog.classify_for_profile(&event, Sport::CrossCountry, &outside)?, EventDisposition::Excluded(EventExclusionReason::OutsideSelectedSport));
    check!(eq; catalog.classify_for_profile(&event, Sport::OutdoorTrack, &outside)?, EventDisposition::Unresolved(EventResolutionReason::ContradictoryDeclarations));
    Ok(())
}

#[test]
fn excluded_disciplines_keep_original_reason_under_selected_profile() -> TestResult {
    let catalog = RecruitingCatalogV1::new();
    let profile = profile(vec![Sport::IndoorTrack], vec![Gender::Girls])?;
    let event = event(
        EventKind::Unmapped {
            label: "1500m Racewalk".into(),
        },
        Gender::Boys,
        "1500m Racewalk",
    )?;
    let expected = EventDisposition::Excluded(EventExclusionReason::CompetitiveWalking);
    check!(eq; catalog.classify(&event, Sport::OutdoorTrack)?, expected);
    check!(eq; catalog.classify_for_profile(&event, Sport::OutdoorTrack, &profile)?, expected);
    Ok(())
}
