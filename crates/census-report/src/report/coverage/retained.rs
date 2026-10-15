use super::contact::ContactPlane;
use super::scan::Scanned;
use super::state::{jurisdiction_of, school_state_index, BucketMap, SchoolSets};
use super::{athletes, classify, CoverageReport, CoverageTotals, ReportError, ReportResult};
use crate::report::{derivation::in_run_scope, Derivation};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::HashSet;

pub(super) fn append(report: &mut CoverageReport, derivation: &Derivation<'_>) -> ReportResult<()> {
    let dataset = derivation.dataset();
    let scanned = Scanned::of(dataset);
    let states = school_state_index(&scanned.schools);
    let mut buckets = raw_buckets(&scanned, &states);
    raw_pr_support(&mut buckets, &scanned, dataset)?;
    let contact = ContactPlane::of(&dataset.coverage_evidence.coach_lanes);
    let all_buckets = UsJurisdiction::ALL
        .iter()
        .copied()
        .map(JurisdictionBucket::from)
        .chain(std::iter::once(JurisdictionBucket::Unplaced));
    let (raw, retained_gaps) = classify::publish(
        buckets,
        &dataset.coverage_evidence.rows,
        &contact,
        all_buckets,
    );
    append_population_notes(report, derivation, &raw)?;
    append_lifecycle(report, derivation);
    append_outside(report, derivation, &raw);
    append_gaps(report, derivation, &retained_gaps);
    report.off_cohort_athletes = scanned
        .athletes
        .iter()
        .filter(|athlete| {
            in_run_scope(
                jurisdiction_of(&states, athlete.school.as_str()),
                derivation.jurisdictions(),
            )
        })
        .filter(|athlete| !super::in_requested_year(athlete, derivation.grad_year()))
        .count();
    Ok(())
}

fn raw_buckets(
    scanned: &Scanned<'_>,
    states: &std::collections::HashMap<&str, Option<UsJurisdiction>>,
) -> BucketMap {
    let mut buckets = BucketMap::new();
    let mut sets = SchoolSets::default();
    let placement = athletes::Placement {
        school_state: states,
        grad_year: None,
    };
    classify::classify_population(scanned, &placement, &mut buckets, &mut sets);
    classify::classify_coaches(scanned.coaches, states, &mut buckets, &mut sets);
    classify::classify_schools(&scanned.schools, &sets, &mut buckets);
    classify::classify_meets(scanned.meets, &mut buckets);
    classify::count_core(&scanned.athletes, states, None, &mut buckets);
    buckets
}

fn raw_pr_support(
    buckets: &mut BucketMap,
    scanned: &Scanned<'_>,
    dataset: &crate::export::ExportDataset,
) -> ReportResult<()> {
    let bests = crate::bests::build_from_dataset(
        dataset,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: None,
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;
    let supported: HashSet<_> = bests
        .iter()
        .filter(|selection| selection.result.adjustment_kind.is_none())
        .map(|selection| selection.athlete_id().as_str())
        .collect();
    let states = school_state_index(&scanned.schools);
    for athlete in scanned.athletes.iter() {
        if supported.contains(athlete.id.as_str()) {
            let bucket = super::state::bucket_mut(
                buckets,
                jurisdiction_of(&states, athlete.school.as_str()),
            );
            bucket.row.with_pr_support = bucket.row.with_pr_support.saturating_add(1);
        }
    }
    Ok(())
}

fn totals<'a>(rows: impl IntoIterator<Item = &'a super::JurisdictionCoverage>) -> CoverageTotals {
    let mut total = CoverageTotals::default();
    for row in rows {
        super::add(&mut total.schools, row.schools);
        super::add(&mut total.athletes, row.athletes);
        super::add(&mut total.coaches, row.coaches);
        super::add(&mut total.meets, row.meets);
        super::add(&mut total.performances, row.performances);
    }
    total
}

fn remainder(
    raw: CoverageTotals,
    published: CoverageTotals,
    outside: CoverageTotals,
) -> ReportResult<CoverageTotals> {
    let count = |raw: usize, published: usize, outside: usize| {
        raw.checked_sub(published)
            .and_then(|remaining| remaining.checked_sub(outside))
            .ok_or_else(|| ReportError::Invariant {
                detail: "retained population does not reconcile with scoped publication".into(),
            })
    };
    Ok(CoverageTotals {
        schools: count(raw.schools, published.schools, outside.schools)?,
        athletes: count(raw.athletes, published.athletes, outside.athletes)?,
        coaches: count(raw.coaches, published.coaches, outside.coaches)?,
        meets: count(raw.meets, published.meets, outside.meets)?,
        performances: count(
            raw.performances,
            published.performances,
            outside.performances,
        )?,
    })
}

fn requested_text(derivation: &Derivation<'_>) -> String {
    UsJurisdiction::CENSUS_SCOPE
        .iter()
        .filter(|jurisdiction| {
            derivation.jurisdictions().is_empty()
                || derivation.jurisdictions().contains(jurisdiction)
        })
        .map(|jurisdiction| jurisdiction.code())
        .collect::<Vec<_>>()
        .join(",")
}

fn append_outside(
    report: &mut CoverageReport,
    derivation: &Derivation<'_>,
    raw: &[super::JurisdictionCoverage],
) {
    for row in raw
        .iter()
        .filter(|row| !in_run_scope(row.jurisdiction, derivation.jurisdictions()))
    {
        if row.schools == 0
            && row.athletes == 0
            && row.coaches == 0
            && row.meets == 0
            && row.performances == 0
            && !has_population_evidence(row.source_population)
            && row.contact.attempted == 0
            && row.contact.pending == 0
        {
            continue;
        }
        report.notes.push(format!("retained outside requested jurisdictions {}: schools={} athletes={} coaches={} meets={} performances={} source_population={} contact_state={} contact_attempted={} contact_pending={} contact_failed={} contact_blocked={} contact_refusal={} contact_exhausted={}",
            row.jurisdiction.code(), row.schools, row.athletes, row.coaches, row.meets,
            row.performances, population_text(row.source_population), super::state_slug(row.state),
            row.contact.attempted, row.contact.pending, row.contact.failed, row.contact.blocked,
            row.contact.accepted_refusal, row.contact.exhausted));
    }
}

fn population_text(population: super::SourcePopulation) -> String {
    let stages = [
        population.registered,
        population.applicable,
        population.attempted,
        population.captured,
        population.parsed,
        population.accepted,
        population.withheld,
    ];
    super::population::STAGES
        .iter()
        .zip(stages)
        .map(|(name, count)| {
            format!(
                "{name}={}",
                count.map_or_else(|| "unmeasured".into(), |value| value.to_string())
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn append_gaps(
    report: &mut CoverageReport,
    derivation: &Derivation<'_>,
    gaps: &[super::CoverageGap],
) {
    for gap in gaps.iter().filter(|gap| gap.count > 0) {
        if gap.class == super::GapClass::MissingPrSupport
            && gap
                .jurisdiction
                .jurisdiction()
                .is_some_and(|value| !value.is_in_census_scope())
        {
            continue;
        }
        let disposition = if in_run_scope(gap.jurisdiction, derivation.jurisdictions()) {
            "before evidence/cohort selection"
        } else {
            "outside requested jurisdictions"
        };
        report.notes.push(format!(
            "retained raw gap {disposition}: jurisdiction={} class={} unit={} count={}",
            gap.jurisdiction.code(),
            gap.class.as_str(),
            gap.unit,
            gap.count
        ));
    }
}

fn has_population_evidence(population: super::SourcePopulation) -> bool {
    [
        population.registered,
        population.applicable,
        population.attempted,
        population.captured,
        population.parsed,
        population.accepted,
        population.withheld,
    ]
    .iter()
    .any(Option::is_some)
}

fn append_lifecycle(report: &mut CoverageReport, derivation: &Derivation<'_>) {
    let rows = &derivation.dataset().coverage_evidence.rows;
    let global = super::population::totals(rows).population(rows.len());
    report.notes.push(format!(
        "publication source lifecycle: {}",
        population_text(report.source_population)
    ));
    report.notes.push(format!(
        "global retained source lifecycle: {}",
        population_text(global)
    ));
    let contact = report.contact;
    report.notes.push(format!("contact attempts: {} attempted ({} reached, {} failed, {} blocked, {} successful-empty, {} accepted refusal, {} exhausted) + {} never attempted",
        contact.attempted, contact.reached, contact.failed, contact.blocked, contact.successful_empty,
        contact.accepted_refusal, contact.exhausted, contact.never_attempted));
    report.notes.push(format!(
        "coverage denominators: {} raw observations -> {} scope-filtered -> {} accepted cohort",
        report.raw_observations, report.scope_filtered, report.accepted_cohort
    ));
    for (reason, count) in &report.terminal_failures {
        report.notes.push(format!(
            "global retained terminal access failure: reason={reason} count={count}"
        ));
    }
}

fn append_population_notes(
    report: &mut CoverageReport,
    derivation: &Derivation<'_>,
    raw: &[super::JurisdictionCoverage],
) -> ReportResult<()> {
    let raw_total = totals(raw.iter());
    let outside = totals(
        raw.iter()
            .filter(|row| !in_run_scope(row.jurisdiction, derivation.jurisdictions())),
    );
    let excluded = remainder(raw_total, report.read, outside)?;
    report.notes.push(format!(
        "publication coverage: evidence_scope={} requested_jurisdictions={} cohort={}",
        derivation.scope().as_str(),
        requested_text(derivation),
        derivation
            .grad_year()
            .map_or_else(|| "all".into(), |year| year.to_string())
    ));
    report.notes.push(format!("retained canonical population reconciliation: raw [{}] = publication [{}] + known outside requested jurisdictions [{}] + evidence/cohort excluded [{}]",
        super::totals_text(&raw_total), super::totals_text(&report.read),
        super::totals_text(&outside), super::totals_text(&excluded)));
    report.notes.push("retained raw PR support outside the national census scope (AK/HI) is unmeasured; no missing-PR-support conclusion is made for those jurisdictions".into());
    report.notes.push("UNPLACED is retained because jurisdiction is unknown; it does not certify membership in any requested jurisdiction or a completed jurisdiction denominator".into());
    report.notes.push("event catalog and terminal access failures retain global frozen evidence; geographic filtering does not certify source completion".into());
    report.notes.push("jurisdiction state describes retained coach-lane status; it does not certify source-population completion or unknown denominator inclusion".into());
    Ok(())
}
