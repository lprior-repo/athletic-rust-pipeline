use super::*;
use census_domain::model::{Confidence, GradYear, Grade, ObservedGrade};

fn spann(accumulated: &Accumulator) -> TestResult<&CanonicalAthlete> {
    Ok(accumulated
        .athletes
        .values()
        .find(|athlete| {
            athlete
                .source
                .as_ref()
                .is_some_and(|source| source.id == "14222592")
        })
        .ok_or("Spann source-owned athlete")?)
}

fn capture_source() -> SourceRef {
    SourceRef::new(
        "milesplit_al",
        Some("https://al.milesplit.com/api/v1/meets/725218/performances".into()),
    )
}

#[test]
fn authentic_published_spann_cohort_is_high_without_a_manufactured_grade() -> TestResult {
    let source = document()?;
    let (accumulated, _) = project(
        source.clone(),
        &[school("Spann school", "38332")],
        metadata()?,
    )?;
    let athlete = spann(&accumulated)?;
    check!(eq; athlete.grad_year, GradYear::CO2027);
    check!(eq; athlete.observed_grades, Vec::new());
    check!(eq;
        athlete.published_graduations,
        vec![PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: capture_source(),
        }]
    );
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::HIGH));
    for (evidence, provider) in athlete
        .evidence
        .iter()
        .zip(source["data"].as_array().ok_or("rows")?)
    {
        let note: Value = serde_json::from_str(evidence.note.as_deref().ok_or("source facts")?)?;
        check!(eq; note["provider"], *provider);
        check!(eq; note["published_grad_year"], 2027);
        check!(eq; evidence.source, capture_source());
    }
    Ok(())
}

#[test]
fn published_spann_year_does_not_make_an_incompatible_canonical_cohort_high() -> TestResult {
    let (accumulated, _) = project(document()?, &[school("Spann school", "38332")], metadata()?)?;
    let mut athlete = spann(&accumulated)?.clone();
    athlete.grad_year = GradYear::new(2028).ok_or("incompatible cohort")?;
    check!(eq; athlete.observed_grades, Vec::new());
    check!(eq; athlete.published_graduations[0].grad_year, GradYear::CO2027);
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::LOW));
    Ok(())
}

#[test]
fn published_spann_year_does_not_override_a_contradicting_grade_observation() -> TestResult {
    let (accumulated, _) = project(document()?, &[school("Spann school", "38332")], metadata()?)?;
    let mut athlete = spann(&accumulated)?.clone();
    let observation = ObservedGrade {
        grade: Grade::new(12).ok_or("published senior grade")?,
        school_year: SchoolYear::new(2025).ok_or("season")?,
        source: SourceRef::new("grade_source", Some("https://example.test/roster".into())),
    };
    athlete.observed_grades.push(observation.clone());
    check!(eq; athlete.observed_grades, vec![observation]);
    check!(eq; athlete.published_graduations[0].grad_year, GradYear::CO2027);
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::LOW));
    Ok(())
}

#[test]
fn missing_or_invalid_owned_years_leave_constructor_and_note_only_cohorts_unknown() -> TestResult {
    for token in [Value::Null, json!("0"), json!("9")] {
        let mut source = document()?;
        source["data"][0]["gradYear"] = token;
        let body = serde_json::to_vec(&source)?;
        let OwnedMeetVerdict::Parsed(page) = parse_owned_meet(&body, 725218) else {
            return Err("retained source-owned rows".into());
        };
        let row = &page.rows[0];
        let bound = school("Spann school", "38332");
        let mut athlete = CanonicalAthlete::new(
            &bound.id,
            "Adelyn Spann",
            GradYear::CO2027,
            row.gender,
            row.source_athlete.clone(),
        );
        let mut evidence = Evidence::parsed(capture_source(), "2026-10-01T23:44:16Z");
        evidence.note = Some(json!({"published_grad_year": 2027}).to_string());
        athlete.evidence.push(evidence);
        record_published_graduation(&mut athlete, row, &capture_source());
        check!(eq; athlete.published_graduations, Vec::new());
        check!(eq; athlete.observed_grades, Vec::new());
        check!(eq; athlete.derived_cohort_confidence(), None);
    }
    Ok(())
}

#[test]
fn owned_year_without_published_status_cannot_become_a_published_claim() {
    let OwnedMeetVerdict::Parsed(page) = parse_owned_meet(TROY, 725218) else {
        panic!("authentic source-owned rows")
    };
    let bound = school("Spann school", "38332");
    for cohort in [
        crate::milesplit::owned::OwnedCohort::Missing,
        crate::milesplit::owned::OwnedCohort::Invalid,
    ] {
        let mut row = page.rows[0].clone();
        row.cohort = cohort;
        let mut athlete = CanonicalAthlete::new(
            &bound.id,
            "Adelyn Spann",
            GradYear::CO2027,
            row.gender,
            row.source_athlete.clone(),
        );
        record_published_graduation(&mut athlete, &row, &capture_source());
        assert_eq!(athlete.published_graduations, Vec::new());
        assert_eq!(athlete.observed_grades, Vec::new());
        assert_eq!(athlete.derived_cohort_confidence(), None);
    }
}
