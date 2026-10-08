use super::invariant;
use super::manifest::Manifest;
use crate::export::ExportDataset;
use crate::report::{ReportResult, Scope};
use crate::workbook::Options;
use census_domain::model::GradYear;

pub(super) fn binding(
    manifest: &Manifest,
    current: &ExportDataset,
    scope: Scope,
    cohort: GradYear,
) -> ReportResult<()> {
    if cohort != GradYear::CO2027 {
        return Err(invariant(format!(
            "the Class-of-2027 cohort is the only cohort that may be sealed; {} is not the census seal cohort",
            cohort.get()
        )));
    }
    let selection = manifest.selection()?;
    complete_selection(&selection, scope, cohort)?;
    same_run(manifest, current)?;
    run_school_year(&selection, current)?;
    same_evidence(manifest, current)?;
    Ok(())
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

fn same_run(manifest: &Manifest, current: &ExportDataset) -> ReportResult<()> {
    if manifest.lineage.run != current.lineage.run
        || manifest.lineage.cohort != current.lineage.cohort
    {
        return Err(invariant(
            "publication is stale or belongs to a different census run".to_string(),
        ));
    }
    Ok(())
}

fn run_school_year(selection: &Options, current: &ExportDataset) -> ReportResult<()> {
    let Some(run) = current.lineage.run else {
        return Ok(());
    };
    if selection.school_year != run.season() {
        return Err(invariant(format!(
            "seal requires contacts assessed in the run's own school year: this generation assesses {} and the run covers {}",
            selection.school_year.get(),
            run.season().get()
        )));
    }
    if current.lineage.cohort != Some(GradYear::CO2027) {
        return Err(invariant(format!(
            "the store's run manifest certifies cohort {}, not {}",
            current
                .lineage
                .cohort
                .map_or_else(|| "nothing".to_string(), |bound| bound.to_string()),
            GradYear::CO2027
        )));
    }
    Ok(())
}

fn same_evidence(manifest: &Manifest, current: &ExportDataset) -> ReportResult<()> {
    if manifest.lineage.store_identity != current.lineage.store_identity
        || manifest.lineage.source_digest != current.lineage.source_digest
        || manifest.lineage.input_digest != current.lineage.input_digest
        || manifest.lineage.schema_revision != current.lineage.schema_revision
        || manifest.lineage.policy_revision != current.lineage.policy_revision
    {
        return Err(invariant(
            "publication is stale or belongs to different source evidence".to_string(),
        ));
    }
    Ok(())
}
