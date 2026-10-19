use super::invariant;
use super::manifest::Manifest;
use crate::export::DatasetLineage;
use crate::report::{ReportResult, Scope};
use crate::workbook::Options;
use census_domain::model::GradYear;

pub(super) fn binding(
    manifest: &Manifest,
    current: &DatasetLineage,
    scope: Scope,
    cohort: GradYear,
) -> ReportResult<()> {
    let selection = manifest.selection()?;
    complete_selection(&selection, scope, cohort)?;
    same_run(manifest, current)?;
    run_school_year(&selection, current, cohort)?;
    same_evidence(manifest, current)?;
    Ok(())
}

pub(super) fn require_selection(
    manifest: &Manifest,
    jurisdictions: &[census_domain::UsJurisdiction],
) -> ReportResult<()> {
    let selection = manifest.selection()?;
    let mut expected: Vec<&str> = effective_jurisdictions(jurisdictions)
        .iter()
        .map(|jurisdiction| jurisdiction.code())
        .collect();
    expected.sort_unstable();
    expected.dedup();
    let mut published: Vec<&str> = effective_jurisdictions(&selection.jurisdictions)
        .iter()
        .map(|jurisdiction| jurisdiction.code())
        .collect();
    published.sort_unstable();
    published.dedup();
    if published != expected {
        return Err(invariant(
            "seal requires the publication's jurisdictions to match the sealed scope".to_string(),
        ));
    }
    Ok(())
}

fn effective_jurisdictions(
    jurisdictions: &[census_domain::UsJurisdiction],
) -> &[census_domain::UsJurisdiction] {
    if jurisdictions.is_empty() {
        &census_domain::UsJurisdiction::CENSUS_SCOPE
    } else {
        jurisdictions
    }
}

fn complete_selection(selection: &Options, scope: Scope, cohort: GradYear) -> ReportResult<()> {
    if selection.limit.is_some() {
        return Err(invariant(
            "seal requires a complete publication of the requested scope and cohort: this generation is limited to a prefix"
                .to_string(),
        ));
    }
    if selection.scope != scope || selection.grad_year != Some(cohort.get()) {
        return Err(invariant(format!(
            "seal requires a complete publication of the requested scope and cohort: this generation is {} of cohort {}",
            selection.scope.as_str(),
            selection
                .grad_year
                .map_or_else(|| "none".to_string(), |year| year.to_string())
        )));
    }
    Ok(())
}

fn same_run(manifest: &Manifest, current: &DatasetLineage) -> ReportResult<()> {
    if manifest.lineage.run != current.run || manifest.lineage.cohort != current.cohort {
        return Err(invariant(
            "publication is stale or belongs to a different census run".to_string(),
        ));
    }
    Ok(())
}

fn run_school_year(
    selection: &Options,
    current: &DatasetLineage,
    cohort: GradYear,
) -> ReportResult<()> {
    let Some(run) = current.run else {
        return Ok(());
    };
    if selection.school_year != run.season() {
        return Err(invariant(format!(
            "seal requires contacts assessed in the run's own school year: this generation assesses {} and the run covers {}",
            selection.school_year.get(),
            run.season().get()
        )));
    }
    if current.cohort != Some(cohort) {
        return Err(invariant(format!(
            "the store's run manifest certifies cohort {}, not {}",
            current
                .cohort
                .map_or_else(|| "nothing".to_string(), |bound| bound.to_string()),
            cohort
        )));
    }
    Ok(())
}

fn same_evidence(manifest: &Manifest, current: &DatasetLineage) -> ReportResult<()> {
    if manifest.lineage.store_identity != current.store_identity
        || manifest.lineage.source_digest != current.source_digest
        || manifest.lineage.schema_revision != current.schema_revision
        || manifest.lineage.policy_revision != current.policy_revision
    {
        return Err(invariant(
            "publication is stale or belongs to different source evidence".to_string(),
        ));
    }
    Ok(())
}
