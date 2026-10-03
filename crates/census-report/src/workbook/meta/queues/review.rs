use super::super::{school_of, subject_of, Family, StoreRows};
use super::{
    queue_row, COHORT_UNVERIFIED, IDENTITY_UNVERIFIED, UNRESOLVED_SCHOOL, UNRESOLVED_VENUE,
};
use census_domain::model::{CanonicalAthlete, CanonicalMeet, CanonicalSchool, Confidence};
use std::collections::HashMap;

pub(super) fn cohort_unverified(
    cohort: &[CanonicalAthlete],
    names: &HashMap<&str, &str>,
) -> Family {
    let mut family = Family::new(COHORT_UNVERIFIED);
    for athlete in cohort {
        if athlete.derived_cohort_confidence() == Some(Confidence::HIGH) {
            continue;
        }
        let subject = subject_of(
            &athlete.canonical_name,
            school_of(names, athlete.school.as_str()),
        );
        let detail = if athlete.observed_grades.is_empty()
            && athlete.published_graduations.is_empty()
        {
            format!(
                "no cohort observation retained; {} evidence row(s), {} source(s)",
                athlete.evidence.len(),
                source_count(athlete)
            )
        } else {
            format!(
                "cohort observations do not confirm canonical year {}; {} published graduation observation(s), {} grade observation(s), {} evidence row(s), {} source(s)",
                athlete.grad_year,
                athlete.published_graduations.len(),
                athlete.observed_grades.len(),
                athlete.evidence.len(),
                source_count(athlete)
            )
        };
        family.push(queue_row(athlete.id.as_str(), subject, detail));
    }
    family
}

pub(super) fn identity_unverified(
    rows: &StoreRows,
    cohort: &[CanonicalAthlete],
    names: &HashMap<&str, &str>,
) -> crate::report::ReportResult<Family> {
    let mut family = Family::new(IDENTITY_UNVERIFIED);
    for athlete in cohort {
        let status = rows
            .identities
            .status(athlete.id.as_str())
            .map_err(census_store::StoreError::from)?;
        if status == census_domain::model::IdentityStatus::Verified {
            continue;
        }
        let subject = subject_of(
            &athlete.canonical_name,
            school_of(names, athlete.school.as_str()),
        );
        family.push(queue_row(
            athlete.id.as_str(),
            subject,
            format!(
                "identity status {}; verification requires a current admissible identity decision",
                status.as_str(),
            ),
        ));
    }
    Ok(family)
}

pub(super) fn unresolved_venues(meets: &[CanonicalMeet]) -> Family {
    let mut family = Family::new(UNRESOLVED_VENUE);
    for meet in meets {
        if meet.state.is_some() {
            continue;
        }
        family.push(queue_row(
            meet.id.as_str(),
            format!("{} ({})", meet.name, meet.date),
            format!(
                "no evidence placed the venue in a jurisdiction; filed under {}",
                census_domain::model::MEET_STATE_UNRESOLVED
            ),
        ));
    }
    family
}

pub(super) fn unresolved_schools(schools: &[CanonicalSchool]) -> Family {
    let mut family = Family::new(UNRESOLVED_SCHOOL);
    for school in schools {
        if school.state.is_some() {
            continue;
        }
        family.push(queue_row(
            school.id.as_str(),
            school.name.clone(),
            "no association or adapter placed the school in a jurisdiction".to_string(),
        ));
    }
    family
}

fn source_count(athlete: &CanonicalAthlete) -> usize {
    let mut sources: Vec<&str> = athlete
        .evidence
        .iter()
        .map(|evidence| evidence.source.id.as_str())
        .chain(
            athlete
                .published_graduations
                .iter()
                .map(|claim| claim.source.id.as_str()),
        )
        .chain(
            athlete
                .observed_grades
                .iter()
                .map(|grade| grade.source.id.as_str()),
        )
        .collect();
    sources.sort_unstable();
    sources.dedup();
    sources.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{
        GradYear, PublishedGraduation, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn athlete(id: &str) -> CanonicalAthlete {
        CanonicalAthlete::new(
            &SchoolId::mint("sch", &["cohort-review-fixture"]),
            "Published Cohort Runner",
            GradYear::CO2027,
            census_domain::model::Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, id),
        )
    }

    #[test]
    fn only_unknown_and_contradictory_cohorts_remain_in_review_when_direct_claims_are_retained(
    ) -> TestResult {
        let unknown = athlete("1001");
        let mut confirmed = athlete("1002");
        confirmed.published_graduations.push(PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::id("owned-api"),
        });
        let mut conflicted = athlete("1003");
        conflicted.published_graduations = vec![
            PublishedGraduation {
                grad_year: GradYear::CO2027,
                source: SourceRef::id("owned-api"),
            },
            PublishedGraduation {
                grad_year: GradYear::new(2028).ok_or("invalid fixture graduation year")?,
                source: SourceRef::id("other-api"),
            },
        ];
        let expected = vec![unknown.id.to_string(), conflicted.id.to_string()];
        let family = cohort_unverified(&[unknown, confirmed, conflicted], &HashMap::new());
        check!(eq;
            family
                .rows
                .iter()
                .map(|row| row.subject_id.clone())
                .collect::<Vec<_>>(),
            expected,
        );
        check!(eq; family.findings, 2);
        Ok(())
    }
}
