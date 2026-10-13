use super::*;
use crate::model::{serialized_bytes, StoreIdentity};
use std::fmt::Debug;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn equal<T: Debug + PartialEq>(actual: T, expected: T) -> TestResult {
    if actual != expected {
        return Err(format!("actual={actual:?}; expected={expected:?}").into());
    }
    Ok(())
}

fn date(value: &str) -> Result<ProfileDate, ProfileError> {
    ProfileDate::parse(value)
}

fn reference() -> Result<ContractRefV1, ProfileError> {
    ContractRefV1::new(1, 1, &"a".repeat(64))
}

fn input() -> TestResult<QueryProfileInputV1> {
    let contract = reference()?;
    Ok(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Colorado,
        ],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: date("2026-10-11")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("invalid fixture school year")?,
        source_plan: contract.clone(),
        acquisition_window: ProfileDateRangeV1::new(date("2026-10-01")?, date("2026-10-13")?)?,
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
    })
}

#[test]
fn reordered_sets_and_duplicates_share_canonical_identity() -> TestResult {
    let baseline = QueryProfileV1::new(input()?)?;
    for rotation in 0..5 {
        let mut raw = input()?;
        raw.jurisdictions.rotate_left(rotation);
        raw.jurisdictions.push(UsJurisdiction::Michigan);
        raw.graduation_years.push(GradYear::CO2027);
        raw.competition_categories.reverse();
        raw.sports.reverse();
        let profile = QueryProfileV1::new(raw)?;
        equal(&profile, &baseline)?;
        equal(profile.id()?, baseline.id()?)?;
        equal(serialized_bytes(&profile)?, serialized_bytes(&baseline)?)?;
    }
    Ok(())
}

#[test]
fn every_semantic_dimension_changes_identity() -> TestResult {
    let original = input()?;
    let baseline = QueryProfileV1::new(original.clone())?.id()?;
    for dimension in 0..21 {
        let mut changed = original.clone();
        let revision = ContractRefV1::new(1, 2, &"a".repeat(64))?;
        match dimension {
            0 => changed.as_of = date("2026-10-12")?,
            1 => changed
                .graduation_years
                .push(GradYear::new(2028).ok_or("year")?),
            2 => changed.regional_membership = RegionalMembershipV1::SourceBackedRegion(revision),
            3 => {
                changed.discovery_priority = DiscoveryPriorityV1::SourceBackedRegionFirst(revision)
            }
            4 => changed.contact_school_year = SchoolYear::new(2027).ok_or("school year")?,
            5 => changed.source_plan = revision,
            6 => changed.completion_contract = revision,
            7 => changed.policies.cohort = revision,
            8 => changed.policies.event = revision,
            9 => changed.policies.identity = revision,
            10 => changed.policies.affiliation = revision,
            11 => changed.policies.contact = revision,
            12 => changed.policies.participation = revision,
            13 => changed.policies.publication = revision,
            14 => changed.include_provisional = ProvisionalInclusionV1::Include,
            15 => {
                changed.acquisition_window =
                    ProfileDateRangeV1::new(date("2026-09-30")?, date("2026-10-13")?)?
            }
            16 => {
                changed.performance_window = PerformanceWindowV1::Inclusive(
                    ProfileDateRangeV1::new(date("2026-01-01")?, date("2026-10-11")?)?,
                )
            }
            17 => changed.jurisdictions.push(UsJurisdiction::Wisconsin),
            18 => changed.sports = vec![Sport::CrossCountry],
            19 => changed.competition_categories = vec![Gender::Mixed],
            _ => changed.source_plan = ContractRefV1::new(1, 1, &"b".repeat(64))?,
        }
        if QueryProfileV1::new(changed)?.id()? == baseline {
            return Err(format!("dimension {dimension} failed to change identity").into());
        }
    }
    Ok(())
}

#[test]
fn exact_date_boundaries_reject_year_only_and_impossible_calendar_days() -> TestResult {
    for valid in ["1900-01-01", "2000-02-29", "2024-02-29", "2100-12-31"] {
        let parsed = date(valid)?;
        equal(parsed.as_str(), valid)?;
        equal(
            serde_json::from_str::<ProfileDate>(&serde_json::to_string(&parsed)?)?,
            parsed,
        )?;
    }
    for invalid in [
        "",
        "2026",
        "1899-12-31",
        "2101-01-01",
        "1900-02-29",
        "2100-02-29",
        "2026-02-29",
        "2026-04-31",
        "2026-13-01",
        "2026-00-01",
        "2026-01-00",
        "2026-1-01",
        "2026-01-1",
        " 2026-01-01",
        "2026-01-01T00:00:00Z",
    ] {
        equal(date(invalid), Err(ProfileError::InvalidDate))?;
        if serde_json::from_str::<ProfileDate>(&serde_json::to_string(invalid)?).is_ok() {
            return Err(format!("serde accepted invalid date {invalid}").into());
        }
    }
    Ok(())
}

#[test]
fn inclusive_windows_and_acquisition_cutoff_are_independent() -> TestResult {
    let mut raw = input()?;
    raw.performance_window = PerformanceWindowV1::Inclusive(ProfileDateRangeV1::new(
        date("2026-01-01")?,
        date("2026-10-11")?,
    )?);
    let profile = QueryProfileV1::new(raw)?;
    for (value, admitted) in [
        ("2025-12-31", false),
        ("2026-01-01", true),
        ("2026-10-11", true),
        ("2026-10-12", false),
    ] {
        equal(profile.admits_performance_date(date(value)?), admitted)?;
    }
    equal(
        profile.input().acquisition_window.through().as_str(),
        "2026-10-13",
    )?;
    equal(
        ProfileDateRangeV1::new(date("2026-10-12")?, date("2026-10-11")?),
        Err(ProfileError::ReversedDateRange),
    )?;
    let mut raw = input()?;
    raw.performance_window = PerformanceWindowV1::Inclusive(ProfileDateRangeV1::new(
        date("2026-01-01")?,
        date("2026-10-12")?,
    )?);
    equal(
        QueryProfileV1::new(raw),
        Err(ProfileError::PerformanceAfterAsOf),
    )?;
    let all = QueryProfileV1::new(input()?)?;
    equal(all.admits_performance_date(date("1900-01-01")?), true)?;
    equal(all.admits_performance_date(date("2026-10-12")?), false)
}

#[test]
fn constructor_reports_exact_set_and_protocol_refusals() -> TestResult {
    for field in [
        ProfileSetField::GraduationYears,
        ProfileSetField::Jurisdictions,
        ProfileSetField::Sports,
        ProfileSetField::CompetitionCategories,
    ] {
        let mut raw = input()?;
        match field {
            ProfileSetField::GraduationYears => raw.graduation_years.clear(),
            ProfileSetField::Jurisdictions => raw.jurisdictions.clear(),
            ProfileSetField::Sports => raw.sports.clear(),
            ProfileSetField::CompetitionCategories => raw.competition_categories.clear(),
        }
        equal(
            QueryProfileV1::new(raw),
            Err(ProfileError::EmptySet { field }),
        )?;
    }
    let mut raw = input()?;
    raw.version = 2;
    equal(
        QueryProfileV1::new(raw),
        Err(ProfileError::UnsupportedVersion { found: 2 }),
    )?;
    let mut raw = input()?;
    raw.sports = vec![Sport::Unknown];
    equal(QueryProfileV1::new(raw), Err(ProfileError::UnknownSport))?;
    let mut raw = input()?;
    raw.competition_categories = vec![Gender::Unknown];
    equal(QueryProfileV1::new(raw), Err(ProfileError::UnknownCategory))?;
    equal(
        ContractRefV1::new(2, 1, &"a".repeat(64)),
        Err(ProfileError::UnsupportedContractVersion { found: 2 }),
    )?;
    equal(
        ContractRefV1::new(1, 0, &"a".repeat(64)),
        Err(ProfileError::ZeroContractRevision),
    )?;
    for invalid in [
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        "é".repeat(32),
    ] {
        equal(ProfileId::parse(&invalid), Err(ProfileError::InvalidDigest))?;
        equal(
            ContractRefV1::new(1, 1, &invalid),
            Err(ProfileError::InvalidDigest),
        )?;
    }
    Ok(())
}

#[test]
fn bounded_container_admission_refuses_before_decoding_excess_element() -> TestResult {
    for (field, maximum, value) in [
        ("graduation_years", 64, serde_json::json!(2027)),
        ("jurisdictions", 51, serde_json::json!("MI")),
        ("sports", 3, serde_json::json!("outdoor_track")),
        ("competition_categories", 3, serde_json::json!("boys")),
    ] {
        let mut raw = serde_json::to_value(input()?)?;
        let array = raw.get_mut(field).ok_or("missing field")?;
        *array = serde_json::Value::Array(vec![value; maximum]);
        QueryProfileV1::new(serde_json::from_value(raw.clone())?)?;
        array_push(
            &mut raw,
            field,
            serde_json::json!({"not_a_domain_element": [1,2,3]}),
        )?;
        let encoded = serde_json::to_vec(&raw)?;
        let error = serde_json::from_slice::<QueryProfileV1>(&encoded)
            .err()
            .ok_or("accepted overflow")?;
        if !error
            .to_string()
            .contains("profile container exceeds element ceiling")
        {
            return Err(format!("excess element was decoded as a domain value: {error}").into());
        }
    }
    Ok(())
}

fn array_push(raw: &mut serde_json::Value, field: &str, value: serde_json::Value) -> TestResult {
    raw.get_mut(field)
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("missing array")?
        .push(value);
    Ok(())
}

#[test]
fn negative_serialized_roundtrips_cannot_bypass_private_invariants() -> TestResult {
    let profile = QueryProfileV1::new(input()?)?;
    let encoded = serde_json::to_vec(&profile)?;
    equal(
        serde_json::from_slice::<QueryProfileV1>(&encoded)?,
        profile.clone(),
    )?;
    equal(
        profile.id()?.as_str(),
        serialized_digest(&profile)?.as_str(),
    )?;
    for (field, value) in [
        ("version", serde_json::json!(0)),
        ("as_of", serde_json::json!("2026")),
        ("sports", serde_json::json!(["unknown"])),
        ("competition_categories", serde_json::json!(["unknown"])),
        ("graduation_years", serde_json::json!([])),
        ("jurisdictions", serde_json::json!([])),
        ("extra", serde_json::json!(1)),
        (
            "source_plan",
            serde_json::json!({"version":1,"revision":0,"digest":"a".repeat(64)}),
        ),
    ] {
        let mut raw = serde_json::to_value(&profile)?;
        raw.as_object_mut()
            .ok_or("missing object")?
            .insert(field.to_string(), value);
        if serde_json::from_value::<QueryProfileV1>(raw).is_ok() {
            return Err(format!("invalid {field} survived private serde boundary").into());
        }
    }
    let mut raw = serde_json::to_value(&profile)?;
    raw.as_object_mut()
        .ok_or("object")?
        .remove("acquisition_window")
        .ok_or("window")?;
    if serde_json::from_value::<QueryProfileV1>(raw).is_ok() {
        return Err("missing mandatory acquisition window accepted".into());
    }
    Ok(())
}

#[test]
fn five_state_multiple_cohort_national49_and_explicit51_stay_distinct() -> TestResult {
    let five = QueryProfileV1::new(input()?)?;
    let mut multiple = input()?;
    multiple
        .graduation_years
        .push(GradYear::new(2028).ok_or("year")?);
    let mut national = input()?;
    national.jurisdictions = UsJurisdiction::CENSUS_SCOPE.to_vec();
    let mut all = input()?;
    all.jurisdictions = UsJurisdiction::ALL.to_vec();
    let profiles = [
        five.clone(),
        QueryProfileV1::new(multiple)?,
        QueryProfileV1::new(national)?,
        QueryProfileV1::new(all)?,
    ];
    for (index, profile) in profiles.iter().enumerate() {
        equal(
            serde_json::from_slice::<QueryProfileV1>(&serde_json::to_vec(profile)?)?,
            profile.clone(),
        )?;
        for other in profiles.iter().skip(index.checked_add(1).ok_or("index")?) {
            if profile.id()? == other.id()? {
                return Err("distinct explicit profiles collided".into());
            }
        }
    }
    let mut priority = input()?;
    priority.discovery_priority = DiscoveryPriorityV1::SourceBackedRegionFirst(reference()?);
    let prioritized = QueryProfileV1::new(priority)?;
    equal(
        &prioritized.input().jurisdictions,
        &five.input().jurisdictions,
    )?;
    let mut region = prioritized.input().clone();
    region.regional_membership = RegionalMembershipV1::SourceBackedRegion(reference()?);
    let regional = QueryProfileV1::new(region)?;
    if regional.id()? == prioritized.id()? {
        return Err("membership filter did not change identity".into());
    }
    equal(five.admits_jurisdiction(UsJurisdiction::Michigan), true)?;
    equal(five.admits_jurisdiction(UsJurisdiction::Ohio), false)
}

#[test]
fn selection_preserves_historical_store_identity_spelling() -> TestResult {
    let owner = StoreIdentity::parse(&"A".repeat(64))?;
    let lower = StoreIdentity::parse(&"a".repeat(64))?;
    let selection = ProfileSelectionV1::new(owner.clone(), QueryProfileV1::new(input()?)?.id()?);
    equal(selection.ensure_store(&owner), Ok(()))?;
    equal(
        selection.ensure_store(&lower),
        Err(ProfileSelectionError::ForeignStore),
    )?;
    let roundtrip: ProfileSelectionV1 = serde_json::from_slice(&serde_json::to_vec(&selection)?)?;
    equal(roundtrip, selection)?;
    Ok(())
}

#[test]
fn constructor_container_limits_are_typed_and_count_raw_elements() -> TestResult {
    for field in [
        ProfileSetField::GraduationYears,
        ProfileSetField::Jurisdictions,
        ProfileSetField::Sports,
        ProfileSetField::CompetitionCategories,
    ] {
        let mut raw = input()?;
        let observed = field.maximum().checked_add(1).ok_or("limit")?;
        match field {
            ProfileSetField::GraduationYears => {
                raw.graduation_years = vec![GradYear::CO2027; observed]
            }
            ProfileSetField::Jurisdictions => {
                raw.jurisdictions = vec![UsJurisdiction::Michigan; observed]
            }
            ProfileSetField::Sports => raw.sports = vec![Sport::CrossCountry; observed],
            ProfileSetField::CompetitionCategories => {
                raw.competition_categories = vec![Gender::Boys; observed]
            }
        }
        equal(
            QueryProfileV1::new(raw),
            Err(ProfileError::TooManyElements {
                field,
                observed,
                max: field.maximum(),
            }),
        )?;
    }
    Ok(())
}

#[test]
fn nested_checked_values_refuse_unknown_fields_and_protocols() -> TestResult {
    for raw in [
        serde_json::json!({"version":2,"revision":1,"digest":"a".repeat(64)}),
        serde_json::json!({"version":1,"revision":0,"digest":"a".repeat(64)}),
        serde_json::json!({"version":1,"revision":1,"digest":"A".repeat(64)}),
        serde_json::json!({"version":1,"revision":1,"digest":"a".repeat(64),"extra":1}),
    ] {
        if serde_json::from_value::<ContractRefV1>(raw).is_ok() {
            return Err("invalid contract reference decoded".into());
        }
    }
    for raw in [
        serde_json::json!({"from":"2026-10-12","through":"2026-10-11"}),
        serde_json::json!({"from":"2026-10-11","through":"2026-10-12","extra":1}),
    ] {
        if serde_json::from_value::<ProfileDateRangeV1>(raw).is_ok() {
            return Err("invalid date range decoded".into());
        }
    }
    let profile = QueryProfileV1::new(input()?)?;
    for field in ["version", "extra"] {
        let selection =
            ProfileSelectionV1::new(StoreIdentity::parse(&"A".repeat(64))?, profile.id()?);
        let mut raw = serde_json::to_value(selection)?;
        raw.as_object_mut()
            .ok_or("object")?
            .insert(field.to_string(), serde_json::json!(2));
        if serde_json::from_value::<ProfileSelectionV1>(raw).is_ok() {
            return Err("unknown profile selection protocol accepted".into());
        }
    }
    Ok(())
}

#[test]
fn calendar_constructor_matches_every_day_in_the_declared_year_envelope() -> TestResult {
    for year in 1900..=2100 {
        for month in 1..=12 {
            for day in 1..=31 {
                let text = format!("{year:04}-{month:02}-{day:02}");
                let expected = chrono::NaiveDate::from_ymd_opt(year, month, day).is_some();
                equal(ProfileDate::parse(&text).is_ok(), expected)?;
            }
        }
    }
    Ok(())
}

#[test]
fn canonical_field_order_and_digest_have_an_independent_byte_oracle() -> TestResult {
    use sha2::{Digest, Sha256};
    let contract = format!(
        "{{\"version\":1,\"revision\":1,\"digest\":\"{}\"}}",
        "a".repeat(64)
    );
    let expected = format!(concat!(
        "{{\"version\":1,\"graduation_years\":[2027],\"jurisdictions\":[\"CO\",\"IL\",\"IA\",\"MI\",\"MN\"],",
        "\"regional_membership\":\"all_selected_jurisdictions\",\"discovery_priority\":\"uniform\",",
        "\"sports\":[\"outdoor_track\",\"indoor_track\",\"cross_country\"],",
        "\"competition_categories\":[\"boys\",\"girls\"],\"as_of\":\"2026-10-11\",",
        "\"performance_window\":\"all_retained_through_as_of\",\"contact_school_year\":2026,",
        "\"source_plan\":{contract},\"acquisition_window\":{{\"from\":\"2026-10-01\",\"through\":\"2026-10-13\"}},",
        "\"completion_contract\":{contract},\"policies\":{{\"cohort\":{contract},\"event\":{contract},",
        "\"identity\":{contract},\"affiliation\":{contract},\"contact\":{contract},",
        "\"participation\":{contract},\"publication\":{contract}}},\"include_provisional\":\"exclude\"}}"
    ), contract = contract);
    let profile = QueryProfileV1::new(input()?)?;
    equal(serialized_bytes(&profile)?.as_slice(), expected.as_bytes())?;
    equal(
        profile.id()?.as_str(),
        format!("{:x}", Sha256::digest(expected.as_bytes())).as_str(),
    )
}
