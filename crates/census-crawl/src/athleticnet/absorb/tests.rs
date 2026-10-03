mod acquisition;
mod profile_rounds;
mod retention;
use super::*;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn conflicting_profile_grades_preserve_both_observations_and_sources() -> TestResult {
    let mut accumulated = Accumulator::default();
    let mut resolved = HashMap::new();
    let mut stats = Stats::default();
    let index = SchoolIndex::from_schools(&[]);
    let target = Target {
        athlete_id: 28127170,
        state: Some(UsJurisdiction::Alaska),
    };
    for (grade, scope) in [(11, Scope::TrackField), (10, Scope::CrossCountry)] {
        let bio: Bio = serde_json::from_value(serde_json::json!({
            "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia",
                "LastName": "Casillas", "Gender": "F", "SchoolID": 13850},
            "grades": {"13850_2026": grade},
            "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
            "allSeasons": []
        }))?;
        let source = SourceRef::new(
            "athleticnet",
            Some(format!(
                "https://www.athletic.net/athlete/28127170/{}",
                scope.parameter()
            )),
        );
        let outcome = absorb(
            &bio,
            scope,
            &target,
            AbsorbContext {
                source: &source,
                observed_on: "2026-09-30",
                index: &index,
                resolved: &mut resolved,
                stats: &mut stats,
                accumulated: &mut accumulated,
            },
        );
        check!(eq;
            outcome?,
            AbsorbOutcome::Complete { rows: 0 }
        );
    }
    let athlete = accumulated.athletes.values().next().ok_or("athlete")?;
    let years: Vec<_> = athlete
        .observed_grades
        .iter()
        .map(|grade| grade.grad_year().map(GradYear::get))
        .collect();
    check!(eq; years, vec![Some(2027), Some(2028)]);
    let urls: Vec<_> = athlete
        .evidence
        .iter()
        .map(|evidence| evidence.source.url.as_deref())
        .collect();
    check!(eq;
        urls,
        vec![
            Some("https://www.athletic.net/athlete/28127170/tf"),
            Some("https://www.athletic.net/athlete/28127170/xc"),
        ]
    );
    Ok(())
}

#[test]
fn unsupported_latest_grade_does_not_discard_earlier_supported_evidence() -> TestResult {
    let bio: Bio = serde_json::from_value(serde_json::json!({
        "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia",
            "LastName": "Casillas", "Gender": "F", "SchoolID": 13850},
        "grades": {"13850_2026": 11, "13850_2041": 12},
        "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
        "allSeasons": []
    }))?;
    let mut accumulated = Accumulator::default();
    let target = Target {
        athlete_id: 28127170,
        state: Some(UsJurisdiction::Alaska),
    };
    let source = SourceRef::new("athleticnet", Some(profile_url(target.athlete_id)));
    let outcome = absorb(
        &bio,
        Scope::TrackField,
        &target,
        AbsorbContext {
            source: &source,
            observed_on: "2026-09-30",
            index: &SchoolIndex::from_schools(&[]),
            resolved: &mut HashMap::new(),
            stats: &mut Stats::default(),
            accumulated: &mut accumulated,
        },
    );
    check!(eq;
        outcome?,
        AbsorbOutcome::Withheld {
            reason: "Unsupported graduation inference"
        }
    );
    check!(accumulated.athletes.is_empty());
    let (cases, rows) = accumulated.unsupported.into_parts();
    check!(eq; cases.len(), 1);
    let years: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let census_domain::model::SourceObservation::Athlete(row) = row else {
                return Err("athlete evidence required".into());
            };
            Ok(row
                .observed_grade
                .ok_or("original grade")?
                .grad_year()
                .map(GradYear::get))
        })
        .collect::<TestResult<_>>()?;
    check!(eq; years, vec![Some(2027), None]);
    Ok(())
}

fn retained_public_profile() -> TestResult<Bio> {
    Ok(serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/athleticnet/samples/live-getathletebiodata-28872883-2026-09-20.json"
    )))?)
}

fn absorb_profile_for(
    bio: &Bio,
    requested_id: u64,
) -> (Accumulator, crate::CrawlResult<AbsorbOutcome>) {
    let mut accumulated = Accumulator::default();
    let target = Target {
        athlete_id: requested_id,
        state: Some(UsJurisdiction::Wisconsin),
    };
    let source = SourceRef::new(
        "athleticnet",
        Some(format!(
            "https://www.athletic.net/athlete/{requested_id}/tf"
        )),
    );
    let outcome = absorb(
        bio,
        Scope::TrackField,
        &target,
        AbsorbContext {
            source: &source,
            observed_on: "2026-09-30",
            index: &SchoolIndex::from_schools(&[]),
            resolved: &mut HashMap::new(),
            stats: &mut Stats::default(),
            accumulated: &mut accumulated,
        },
    );
    (accumulated, outcome)
}

#[test]
fn a_returned_profile_cannot_be_attributed_to_another_requested_subject() -> TestResult {
    let (accumulated, outcome) = absorb_profile_for(&retained_public_profile()?, 28127170);
    let (url, detail) = match outcome {
        Err(crate::CrawlError::Schema { url, detail }) => (url, detail),
        other => {
            return Err(format!("foreign profile rejected with schema error: {other:?}").into())
        }
    };
    check!(eq;
        url,
        "https://www.athletic.net/athlete/28127170/tf#athlete/IDAthlete"
    );
    check!(eq;
        detail,
        "Requested athlete 28127170 but returned athlete 28872883"
    );
    check!(eq; accumulated.profile_reviews.len(), 1);
    check!(accumulated.profile_observations.is_empty());
    check!(eq; accumulated.unsupported.len(), 0);
    check!(
        accumulated.athletes.is_empty(),
        "the response belongs to 28872883"
    );
    check!(accumulated.performances.is_empty());
    check!(accumulated.schools.is_empty());
    Ok(())
}

#[test]
fn unknown_gender_keeps_published_grade_and_source_owner_without_canonical_admission() -> TestResult
{
    let mut bio = retained_public_profile()?;
    bio.athlete.gender.clear();
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    check!(eq;
        outcome?,
        AbsorbOutcome::Withheld {
            reason: "Unknown published gender"
        }
    );
    check!(accumulated.athletes.is_empty());
    let (_, observations) = accumulated.unsupported.into_parts();
    let grades: Vec<_> = observations
        .iter()
        .map(|observation| {
            let census_domain::model::SourceObservation::Athlete(row) = observation else {
                return Err("source athlete observation".into());
            };
            let grade = row.observed_grade.as_ref().ok_or("published grade")?;
            Ok((
                row.source_athlete_id.as_str(),
                grade.grade.get(),
                grade.school_year.get(),
            ))
        })
        .collect::<TestResult<_>>()?;
    check!(eq; grades, [("28872883", 10, 2024), ("28872883", 11, 2025)]);
    Ok(())
}

#[test]
fn admitted_profile_has_positive_parsed_support_for_source_bound_identity() -> TestResult {
    let (accumulated, outcome) = absorb_profile_for(&retained_public_profile()?, 28872883);
    check!(matches!(
        outcome?,
        AbsorbOutcome::Complete { .. } | AbsorbOutcome::Partial { .. }
    ));
    let athlete = accumulated
        .athletes
        .values()
        .next()
        .ok_or("profile subject")?;
    let mut index = census_domain::model::AthleteIdentityIndex::default();
    index.observe(athlete)?;
    check!(index.isolated_source(&athlete.id.cast()));
    Ok(())
}
