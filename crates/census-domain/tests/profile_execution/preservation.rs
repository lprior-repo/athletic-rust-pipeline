use super::super::*;
use super::common::{date, equal, input, TestResult};

#[test]
fn compiled_capability_preserves_every_nondefault_selected_dimension() -> TestResult {
    let mut raw = input()?;
    raw.graduation_years = vec![
        GradYear::new(2030).ok_or("graduation year")?,
        GradYear::new(2028).ok_or("graduation year")?,
        GradYear::CO2027,
    ];
    raw.jurisdictions = vec![
        UsJurisdiction::Michigan,
        UsJurisdiction::Minnesota,
        UsJurisdiction::Iowa,
        UsJurisdiction::Illinois,
        UsJurisdiction::Colorado,
    ];
    raw.sports = vec![Sport::CrossCountry, Sport::IndoorTrack, Sport::OutdoorTrack];
    raw.competition_categories = vec![Gender::Mixed, Gender::Girls, Gender::Boys];
    raw.as_of = date("2028-06-01")?;
    raw.performance_window = PerformanceWindowV1::Inclusive(ProfileDateRangeV1::new(
        date("2025-01-01")?,
        date("2028-05-31")?,
    )?);
    raw.acquisition_window = ProfileDateRangeV1::new(date("2028-01-01")?, date("2028-05-30")?)?;
    raw.contact_school_year = SchoolYear::new(2027).ok_or("contact school year")?;
    let profile = QueryProfileV1::new(raw)?;
    let bytes = serialized_bytes(&profile)?;
    let id = profile.id()?;
    let compiled = compile_profile(&profile)?;
    equal(std::ptr::eq(compiled.profile(), &profile), true)?;
    equal(serialized_bytes(compiled.profile())?, bytes)?;
    equal(compiled.profile().id()?, id)?;
    equal(compiled.profile().input().graduation_years.len(), 3)?;
    equal(compiled.profile().input().sports.len(), 3)?;
    equal(compiled.profile().input().competition_categories.len(), 3)?;
    equal(compiled.profile().input().contact_school_year.get(), 2027)
}

#[test]
fn national49_and_every_in_scope_single_jurisdiction_compile() -> TestResult {
    let mut raw = input()?;
    raw.jurisdictions = UsJurisdiction::CENSUS_SCOPE.to_vec();
    let national = QueryProfileV1::new(raw)?;
    equal(
        compile_profile(&national)?
            .profile()
            .input()
            .jurisdictions
            .len(),
        49,
    )?;
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        let mut raw = input()?;
        raw.jurisdictions = vec![jurisdiction];
        let profile = QueryProfileV1::new(raw)?;
        equal(
            compile_profile(&profile)?
                .profile()
                .input()
                .jurisdictions
                .as_slice(),
            &[jurisdiction],
        )?;
    }
    Ok(())
}

#[test]
fn explicit51_remains_a_distinct_serializable_raw_profile() -> TestResult {
    let mut raw = input()?;
    raw.jurisdictions = UsJurisdiction::ALL.to_vec();
    let all = QueryProfileV1::new(raw)?;
    let bytes = serialized_bytes(&all)?;
    let roundtrip: QueryProfileV1 = serde_json::from_slice(&bytes)?;
    equal(&roundtrip, &all)?;
    equal(all.input().jurisdictions.len(), 51)?;
    if !matches!(
        compile_profile(&all),
        Err(ProfileExecutionError::UnsupportedJurisdiction {
            jurisdiction: UsJurisdiction::Alaska,
        })
    ) {
        return Err("explicit51 did not retain deterministic interim refusal".into());
    }
    equal(serialized_bytes(&all)?, bytes)
}

#[test]
fn reordered_sets_keep_one_original_profile_identity_after_compilation() -> TestResult {
    let mut raw = input()?;
    raw.jurisdictions = vec![UsJurisdiction::Michigan, UsJurisdiction::Iowa];
    raw.graduation_years = vec![GradYear::CO2027, GradYear::new(2028).ok_or("year")?];
    let original = QueryProfileV1::new(raw.clone())?;
    raw.jurisdictions.reverse();
    raw.jurisdictions.push(UsJurisdiction::Michigan);
    raw.graduation_years.reverse();
    let reordered = QueryProfileV1::new(raw)?;
    equal(
        compile_profile(&original)?.profile().id()?,
        compile_profile(&reordered)?.profile().id()?,
    )?;
    equal(serialized_bytes(&original)?, serialized_bytes(&reordered)?)
}
