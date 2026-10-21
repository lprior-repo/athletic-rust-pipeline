use crate::restate_services::wire::NationalRequest;
use crate::restate_services::JobError;
use census_domain::model::{CensusRun, GradYear, RetainedConflict, ReviewCase};
use census_domain::profile_execution::validate_current_selected_qualification_profile;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use census_reconcile::identity::admitted_scope;
use census_report::{report::Scope, workbook};
use census_store::{Store, StoreSnapshot, Table};

pub(super) fn inspect(
    store: &Store,
    request: &NationalRequest,
    cohort: GradYear,
) -> Result<String, JobError> {
    let selection = request.export_job.require_selected_publication();
    workbook::options_for_selected_request(
        store,
        selection,
        &workbook::Options {
            grad_year: Some(cohort.get()),
            out: None,
            limit: None,
            scope: Scope::AllSources,
            jurisdictions: request.jurisdictions.clone(),
            school_year: request.season,
        },
    )?;
    let snapshot = store.snapshot();
    let profile = snapshot.selected_profile(selection)?;
    validate_current_selected_qualification_profile(&profile).map_err(|error| {
        JobError::Terminal {
            message: error.to_string(),
        }
    })?;
    require_run(
        &snapshot,
        request,
        cohort,
        selection.store_identity().as_str(),
    )?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or_else(|| {
            refused("current National reconciliation requires accepted GraphV3 authority")
        })?;
    if accepted.qualification_version()
        != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
    {
        return Err(refused(
            "current National reconciliation refuses historical identity authority",
        ));
    }
    let conflicts = snapshot.for_each_merged::<RetainedConflict>(Table::Conflicts, |_| Ok(()))?;
    let reviews = snapshot.for_each_merged::<ReviewCase>(Table::ReviewCases, |_| Ok(()))?;
    Ok(format!(
        "selected_graph_v3 profile={} snapshot={} head_sequence={} head_digest={} public_ids={} history_records={} retained_conflicts={} retained_reviews={} source_identity_derivation=selected_workbook",
        profile.id().map_err(|error| JobError::Terminal { message: error.to_string() })?,
        accepted.snapshot_sequence(), accepted.head().sequence, accepted.head().digest,
        accepted.public_ids().count(), accepted.history_records().len(), conflicts, reviews,
    ))
}

fn require_run(
    snapshot: &StoreSnapshot<'_>,
    request: &NationalRequest,
    cohort: GradYear,
    store_identity: &str,
) -> Result<(), JobError> {
    let manifest = snapshot.run_manifest()?.ok_or_else(|| {
        refused("current National reconciliation requires its bound run manifest")
    })?;
    let run = CensusRun::new(request.season, request.revision.get())
        .ok_or_else(|| refused("current National reconciliation has an invalid run"))?;
    let jurisdictions = admitted_scope(&request.jurisdictions);
    if manifest.store_identity != store_identity
        || manifest.run != run
        || manifest.cohort != cohort
        || manifest.jurisdictions.len() != jurisdictions.len()
        || !manifest
            .jurisdictions
            .iter()
            .all(|site| jurisdictions.contains(site))
    {
        return Err(refused(
            "current National reconciliation differs from its bound run manifest",
        ));
    }
    Ok(())
}

fn refused(message: &str) -> JobError {
    JobError::Terminal {
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests;
