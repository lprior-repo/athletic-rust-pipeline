use super::contact::ContactPlane;
use super::state::{BucketMap, SchoolSets};
use super::{athletes, classify, pr_support, retained, CoverageReport, CoverageTotals};
use crate::report::{Derivation, ReportResult};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::HashSet;

pub(super) fn report(derivation: &Derivation<'_>) -> ReportResult<CoverageReport> {
    let dataset = derivation.dataset();
    validate_stage_capacity(&dataset.coverage_evidence.rows)?;
    let admission = crate::report::EventAdmission::of(dataset)?;
    let requested = requested(derivation);
    let contact = contact_plane(derivation, &requested);
    let published = requested
        .iter()
        .copied()
        .map(JurisdictionBucket::from)
        .chain(std::iter::once(JurisdictionBucket::Unplaced));
    let (jurisdictions, gaps) = classify::publish(
        classify_selected(derivation),
        &dataset.coverage_evidence.rows,
        &contact,
        published,
    );
    let mut report = CoverageReport {
        grad_year: derivation.grad_year(),
        event_catalog: admission.counts().clone(),
        source_population: source_population(&jurisdictions)?,
        state_counts: super::state_histogram(&jurisdictions),
        jurisdictions,
        gaps,
        read: selected_totals(derivation),
        off_cohort_athletes: 0,
        notes: Vec::new(),
        contact: contact.totals(),
        terminal_failures: classify::compute_terminal_failures(dataset),
        raw_observations: dataset.athletes.len(),
        scope_filtered: derivation.scoped_athletes(),
        accepted_cohort: derivation
            .athletes()
            .iter()
            .filter(|athlete| crate::report::is_core_evidenced(*athlete))
            .count(),
    };
    apply_pr_support(&mut report, derivation)?;
    retained::append(&mut report, derivation)?;
    report.reconcile()?;
    Ok(report)
}

fn requested(derivation: &Derivation<'_>) -> Vec<UsJurisdiction> {
    UsJurisdiction::CENSUS_SCOPE
        .iter()
        .copied()
        .filter(|jurisdiction| {
            derivation.jurisdictions().is_empty()
                || derivation.jurisdictions().contains(jurisdiction)
        })
        .collect()
}

fn contact_plane(derivation: &Derivation<'_>, requested: &[UsJurisdiction]) -> ContactPlane {
    let lanes = derivation
        .dataset()
        .coverage_evidence
        .coach_lanes
        .iter()
        .filter(|lane| requested.contains(&lane.jurisdiction()));
    let mut contact = ContactPlane::of(lanes);
    contact.declare_scope(requested.iter().copied());
    contact
}

fn classify_selected(derivation: &Derivation<'_>) -> BucketMap {
    let mut buckets = BucketMap::new();
    let mut sets = SchoolSets::default();
    let states = derivation.school_state();
    let placement = athletes::Placement {
        school_state: states,
        grad_year: None,
    };
    let coach_schools = derivation
        .coaches()
        .iter()
        .map(|row| row.school.as_str())
        .collect();
    athletes::classify(
        derivation.athletes(),
        &placement,
        &coach_schools,
        &mut buckets,
        &mut sets.with_athletes,
    );
    classify_performances(derivation, &placement, &mut buckets);
    classify::classify_coaches(derivation.coaches(), states, &mut buckets, &mut sets);
    classify::classify_schools(derivation.schools(), &sets, &mut buckets);
    classify::classify_meets(derivation.meets(), &mut buckets);
    classify::count_core(derivation.athletes(), states, None, &mut buckets);
    buckets
}

fn classify_performances(
    derivation: &Derivation<'_>,
    placement: &athletes::Placement<'_, '_>,
    buckets: &mut BucketMap,
) {
    let subjects = derivation
        .athletes()
        .iter()
        .map(|row| row.id.as_str())
        .collect();
    let events = derivation
        .events()
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let unmapped = derivation
        .events()
        .iter()
        .filter(|row| matches!(row.kind, census_domain::model::EventKind::Unmapped { .. }))
        .map(|row| row.id.as_str().to_string())
        .collect();
    let (tallies, orphan) = athletes::tally_performances(
        derivation.performances().iter().copied(),
        &subjects,
        &events,
        &unmapped,
        derivation.athlete_aliases(),
    );
    athletes::classify_performances(derivation.athletes(), placement, &tallies, &orphan, buckets);
}

fn source_population(
    rows: &[super::JurisdictionCoverage],
) -> ReportResult<super::SourcePopulation> {
    let expected: Vec<_> = rows
        .iter()
        .filter(|row| row.jurisdiction.jurisdiction().is_some())
        .collect();
    let mut sums = [Some(0_usize); 7];
    for row in &expected {
        let population = row.source_population;
        let stages = [
            population.registered,
            population.applicable,
            population.attempted,
            population.captured,
            population.parsed,
            population.accepted,
            population.withheld,
        ];
        for (sum, value) in sums.iter_mut().zip(stages) {
            *sum = match (*sum, value) {
                (Some(sum), Some(value)) => {
                    Some(sum.checked_add(value).ok_or_else(stage_capacity_error)?)
                }
                _ => None,
            };
        }
    }
    let value = |index| {
        (!expected.is_empty())
            .then(|| sums.get(index).copied().flatten())
            .flatten()
    };
    Ok(super::SourcePopulation {
        registered: value(0),
        applicable: value(1),
        attempted: value(2),
        captured: value(3),
        parsed: value(4),
        accepted: value(5),
        withheld: value(6),
    })
}

fn selected_totals(derivation: &Derivation<'_>) -> CoverageTotals {
    CoverageTotals {
        schools: derivation.schools().len(),
        athletes: derivation.athletes().len(),
        coaches: derivation.coaches().len(),
        meets: derivation.meets().len(),
        performances: derivation.performances().len(),
    }
}

fn apply_pr_support(report: &mut CoverageReport, derivation: &Derivation<'_>) -> ReportResult<()> {
    let bests = crate::bests::build_from_dataset(
        derivation.dataset(),
        &crate::bests::Options {
            scope: derivation.scope(),
            grad_year: derivation.grad_year(),
            limit: None,
            jurisdictions: derivation.jurisdictions().to_vec(),
        },
    )?;
    let supported: HashSet<&str> = bests
        .iter()
        .filter(|selection| selection.result.adjustment_kind.is_none())
        .map(|selection| selection.athlete_id().as_str())
        .collect();
    pr_support::tally_pr_support(
        report,
        derivation.dataset(),
        derivation.grad_year(),
        &supported,
    );
    pr_support::write_pr_gaps(report);
    Ok(())
}

fn validate_stage_capacity(rows: &[census_domain::model::CoverageRow]) -> ReportResult<()> {
    let mut sums = [0_usize; 7];
    for row in rows {
        for (sum, stage) in sums.iter_mut().zip(super::population::STAGES) {
            if let Some(value) = row.metrics.get(stage) {
                let value = usize::try_from(*value).map_err(|_| stage_capacity_error())?;
                *sum = sum.checked_add(value).ok_or_else(stage_capacity_error)?;
            }
        }
    }
    Ok(())
}

fn stage_capacity_error() -> super::ReportError {
    super::ReportError::Invariant {
        detail: "coverage source stage total exceeds representable capacity".into(),
    }
}
