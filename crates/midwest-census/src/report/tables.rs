//! Per-table aggregates: meet coverage, school-table counts, and the `ALL` row.

use super::notes::{add, bump};
use super::rows::RowCounts;
use super::{MeetCoverage, StateCensus};
use census_domain::model::{CanonicalMeet, CanonicalSchool, SourceNamespace};
use census_domain::{JurisdictionBucket, MeetState};
use std::collections::BTreeMap;

/// Meet-table coverage, one pass over the merged meet rows.
pub(super) fn meet_coverage(meets: &[CanonicalMeet]) -> MeetCoverage {
    let mut coverage = MeetCoverage::default();
    for meet in meets {
        bump(&mut coverage.total);
        // An unplaced venue stays in the unresolved bucket the pre-cutover reports published, and
        // the key prints exactly that label.
        bump(
            coverage
                .by_state
                .entry(MeetState::from(meet.state))
                .or_default(),
        );
        let names_athletic_net = meet.source_identities.iter().any(|identity| {
            matches!(
                identity.namespace,
                SourceNamespace::LegacyAthleticNet { .. }
            )
        });
        if names_athletic_net {
            bump(&mut coverage.with_athletic_net_id);
        }
        for identity in &meet.source_identities {
            if let SourceNamespace::TimerMeet { provider } = &identity.namespace {
                let slug = format!("timer_meet:{provider}");
                bump(coverage.by_provider.entry(slug).or_default());
            }
        }
        let earliest = coverage.first_date.get_or_insert_with(|| meet.date.clone());
        if meet.date < *earliest {
            *earliest = meet.date.clone();
        }
        let latest = coverage.last_date.get_or_insert_with(|| meet.date.clone());
        if meet.date > *latest {
            *latest = meet.date.clone();
        }
    }
    coverage
}

/// School counts per jurisdiction bucket, from the school table rather than from athlete-derived
/// buckets. A school whose row carries no state counts under the unplaced bucket, exactly where the
/// per-state row for it prints.
pub(super) fn schools_by_state(schools: &[CanonicalSchool]) -> BTreeMap<JurisdictionBucket, usize> {
    let mut counts: BTreeMap<JurisdictionBucket, usize> = BTreeMap::new();
    for school in schools {
        bump(
            counts
                .entry(JurisdictionBucket::from(school.state))
                .or_default(),
        );
    }
    counts
}

/// Number of `(state, normalized_name)` pairs shared by more than one school row.
pub(super) fn duplicate_school_names(schools: &[CanonicalSchool]) -> usize {
    let mut name_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for school in schools {
        let key = (
            school
                .state
                .map_or(String::new(), |state| state.code().to_string()),
            school.normalized_name.clone(),
        );
        bump(name_counts.entry(key).or_default());
    }
    name_counts.values().filter(|count| **count > 1).count()
}

/// The `ALL` row: state buckets summed, school and coach totals taken from the tables.
pub(super) fn totals_of(
    by_state: &BTreeMap<JurisdictionBucket, StateCensus>,
    counts: &RowCounts,
) -> StateCensus {
    let mut totals = StateCensus::default();
    for entry in by_state.values() {
        add(&mut totals.athletes, entry.athletes);
        add(&mut totals.class_of_2027, entry.class_of_2027);
        add(&mut totals.class_of_2027_boys, entry.class_of_2027_boys);
        add(&mut totals.class_of_2027_girls, entry.class_of_2027_girls);
        add(
            &mut totals.class_of_2027_unknown_gender,
            entry.class_of_2027_unknown_gender,
        );
        add(
            &mut totals.class_of_2027_with_profile_url,
            entry.class_of_2027_with_profile_url,
        );
        add(
            &mut totals.class_of_2027_with_grad_year_evidence,
            entry.class_of_2027_with_grad_year_evidence,
        );
        add(
            &mut totals.class_of_2027_multisource,
            entry.class_of_2027_multisource,
        );
        add(
            &mut totals.class_of_2027_with_coach,
            entry.class_of_2027_with_coach,
        );
        add(
            &mut totals.class_of_2027_with_coach_email,
            entry.class_of_2027_with_coach_email,
        );
    }
    totals.state = crate::report::RowLabel::Total;
    totals.schools = counts.schools;
    totals.coaches = counts.coaches;
    totals.coaches_with_email = counts.coaches_with_email;
    totals
}
