//! The coverage rows one derivation pass writes: one per jurisdiction from the reconciled coverage
//! report, and one per source namespace from the identities the same pass read.
//!
//! Both come from computations the census publishes elsewhere — the report the coverage command prints
//! and the identity join the index itself derives — so a reader of the store's coverage table and a
//! reader of the workbook cannot be shown different numbers.

use std::collections::BTreeMap;

use census_domain::model::{CoverageRow, CoverageScope, GradYear, SourceNamespace, SourceObjectIdentity};

use census_report::report::{coverage_report, ReportResult};
use census_store::Store;

/// Coverage rows: one per jurisdiction from the reconciled coverage report, one per source namespace
/// from the identities this pass read.
pub(super) fn coverage_rows(
    store: &Store,
    identities: &[SourceObjectIdentity],
) -> ReportResult<Vec<CoverageRow>> {
    let report = coverage_report(store, Some(GradYear::CO2027.get()))?;
    let mut rows = Vec::with_capacity(report.jurisdictions.len().saturating_add(identities.len()));
    for jurisdiction in &report.jurisdictions {
        rows.push(
            CoverageRow::new(
                CoverageScope::Jurisdiction,
                jurisdiction.jurisdiction.to_string(),
            )
            .with("schools", count(jurisdiction.schools))
            .with(
                "schools_with_athletes",
                count(jurisdiction.schools_with_athletes),
            )
            .with("cohort_athletes", count(jurisdiction.athletes))
            .with("boys", count(jurisdiction.boys))
            .with("girls", count(jurisdiction.girls))
            .with("grad_verified", count(jurisdiction.grad_verified))
            .with("grad_unresolved", count(jurisdiction.grad_unresolved))
            .with("multisource", count(jurisdiction.multisource))
            .with("with_performance", count(jurisdiction.with_performance))
            .with("with_profile_url", count(jurisdiction.with_profile_url))
            .with(
                "schools_with_tf_coach",
                count(jurisdiction.schools_with_tf_coach),
            )
            .with(
                "schools_with_xc_coach",
                count(jurisdiction.schools_with_xc_coach),
            )
            .with(
                "schools_with_coach_email",
                count(jurisdiction.schools_with_coach_email),
            )
            .with("meets", count(jurisdiction.meets))
            .with("coaches", count(jurisdiction.coaches))
            .with("coaches_with_email", count(jurisdiction.coaches_with_email)),
        );
    }
    rows.extend(source_coverage(identities));
    Ok(rows)
}

/// One coverage row per source namespace: how many source objects it contributes, per canonical
/// table.
pub(super) fn source_coverage(identities: &[SourceObjectIdentity]) -> Vec<CoverageRow> {
    let mut by_source: BTreeMap<&SourceNamespace, BTreeMap<String, u64>> = BTreeMap::new();
    for identity in identities {
        let metrics = by_source.entry(&identity.namespace).or_default();
        bump(metrics, "identities");
        bump(metrics, identity.entity.slug());
    }
    by_source
        .into_iter()
        .map(|(namespace, metrics)| {
            let source = namespace.to_string();
            CoverageRow {
                id: format!("source:{source}"),
                scope: CoverageScope::Source,
                subject: source,
                metrics,
            }
        })
        .collect()
}

/// Count one occurrence, saturating rather than wrapping.
fn bump(metrics: &mut BTreeMap<String, u64>, metric: &str) {
    let slot = metrics.entry(metric.to_string()).or_default();
    *slot = slot.saturating_add(1);
}

/// A count as the store publishes it. A `usize` larger than `u64` cannot exist on a 64-bit target;
/// where it could, the measurement saturates instead of wrapping into a smaller number.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
