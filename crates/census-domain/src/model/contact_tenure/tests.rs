use super::*;

fn year(value: i16) -> Result<SchoolYear, &'static str> {
    SchoolYear::new(value).ok_or("invalid school-year fixture")
}

fn evidence(tenure: CoachTenure) -> CoachTenureEvidence {
    CoachTenureEvidence {
        tenure,
        source: SourceRef::new("synthetic-directory", None),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-01T00:00:00Z".into(),
        statement: "Synthetic tenure statement".into(),
    }
}

#[test]
fn current_means_current_in_the_requested_school_year() -> Result<(), Box<dyn std::error::Error>> {
    let fact = evidence(CoachTenure::Current {
        school_year: year(2026)?,
    });
    check!(eq; assess_coach_tenure([&fact], year(2026)?), Ok(fact.tenure));
    check!(eq; assess_coach_tenure([&fact], year(2025)?),
Ok(CoachTenure::Unknown));
    check!(eq; assess_coach_tenure([&fact], year(2027)?),
Ok(CoachTenure::Unknown));
    Ok(())
}

#[test]
fn former_claims_preserve_the_latest_known_year_without_using_future_facts(
) -> Result<(), Box<dyn std::error::Error>> {
    let past = evidence(CoachTenure::Former {
        last_school_year: Some(year(2024)?),
    });
    let recent = evidence(CoachTenure::Former {
        last_school_year: Some(year(2025)?),
    });
    let undated = evidence(CoachTenure::Former {
        last_school_year: None,
    });
    let future = evidence(CoachTenure::Former {
        last_school_year: Some(year(2027)?),
    });
    for facts in [
        [&past, &recent, &undated, &future],
        [&future, &undated, &recent, &past],
    ] {
        check!(eq; assess_coach_tenure(facts, year(2026)?), Ok(recent.tenure));
    }
    check!(eq; assess_coach_tenure([&future], year(2026)?),
Ok(CoachTenure::Unknown));
    Ok(())
}

#[test]
fn contradictory_current_and_former_claims_never_collapse_to_unknown(
) -> Result<(), Box<dyn std::error::Error>> {
    let current = evidence(CoachTenure::Current {
        school_year: year(2026)?,
    });
    let former = evidence(CoachTenure::Former {
        last_school_year: Some(year(2025)?),
    });
    for facts in [[&current, &former], [&former, &current]] {
        check!(eq; assess_coach_tenure(facts, year(2026)?),
    Err(TenureAssessmentError::Conflict));
    }
    Ok(())
}

#[test]
fn absent_tenure_does_not_erase_an_explicit_claim_or_create_one(
) -> Result<(), Box<dyn std::error::Error>> {
    let unknown = evidence(CoachTenure::Unknown);
    let current = evidence(CoachTenure::Current {
        school_year: year(2026)?,
    });
    check!(eq; assess_coach_tenure([], year(2026)?),
Ok(CoachTenure::Unknown));
    check!(eq; assess_coach_tenure([&unknown], year(2026)?),
Ok(CoachTenure::Unknown));
    for facts in [[&unknown, &current], [&current, &unknown]] {
        check!(eq; assess_coach_tenure(facts, year(2026)?), Ok(current.tenure));
    }
    Ok(())
}

#[test]
fn malformed_capture_metadata_cannot_qualify_a_tenure_claim(
) -> Result<(), Box<dyn std::error::Error>> {
    let valid = evidence(CoachTenure::Current {
        school_year: year(2026)?,
    });
    let mut malformed = Vec::new();
    for digest in ["a".repeat(63), "g".repeat(64), "a".repeat(65)] {
        let mut fact = valid.clone();
        fact.source_sha256 = digest;
        malformed.push(fact);
    }
    for timestamp in ["recently", "2026-09-01", "2026-02-29T00:00:00Z"] {
        let mut fact = valid.clone();
        fact.retrieved_at = timestamp.into();
        malformed.push(fact);
    }
    for fact in &malformed {
        check!(
            matches!(
                assess_coach_tenure([fact], year(2026)?),
                Err(TenureAssessmentError::InvalidEvidence { .. })
            ),
            "accepted {fact:?}"
        );
    }
    Ok(())
}

#[test]
fn empty_or_whitespace_claim_fields_are_refused() -> Result<(), Box<dyn std::error::Error>> {
    let valid = evidence(CoachTenure::Current {
        school_year: year(2026)?,
    });
    for empty in ["", " \t "] {
        for field in ["source.id", "source_sha256", "retrieved_at", "statement"] {
            let mut fact = valid.clone();
            match field {
                "source.id" => fact.source.id = empty.into(),
                "source_sha256" => fact.source_sha256 = empty.into(),
                "retrieved_at" => fact.retrieved_at = empty.into(),
                "statement" => fact.statement = empty.into(),
                _ => return Err("unknown tenure fixture field".into()),
            }
            check!(
                matches!(
                    assess_coach_tenure([&fact], year(2026)?),
                    Err(TenureAssessmentError::InvalidEvidence { .. })
                ),
                "accepted {field}"
            );
        }
    }
    Ok(())
}

#[test]
fn the_statement_byte_limit_accepts_the_boundary_and_rejects_overflow(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut fact = evidence(CoachTenure::Current {
        school_year: year(2026)?,
    });
    fact.statement = "é".repeat(256);
    check!(eq; assess_coach_tenure([&fact], year(2026)?), Ok(fact.tenure));
    fact.statement.push('x');
    check!(eq; validate_tenure_evidence(&fact),
    Err(TenureValidation::TooLong {
        field: "statement",
        limit: 512
    }));
    Ok(())
}

#[test]
fn the_contact_school_year_changes_in_august_not_on_january_first(
) -> Result<(), Box<dyn std::error::Error>> {
    for (date, expected) in [
        ("2026-01-01", 2025),
        ("2026-07-31", 2025),
        ("2026-08-01", 2026),
        ("2026-12-31", 2026),
        ("2027-01-01", 2026),
    ] {
        check!(eq; SchoolYear::from_date(date), Some(year(expected)?));
    }
    for invalid in [
        "2026-02-29",
        "2026-13-01",
        "1900-01-01",
        "2101-08-01",
        "invalid",
    ] {
        check!(eq; SchoolYear::from_date(invalid), None);
    }
    Ok(())
}
