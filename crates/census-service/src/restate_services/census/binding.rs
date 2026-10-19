use super::super::wire::{BindRunReply, BindRunRequest};
use super::super::JobError;
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};
use census_report::{report::Scope, workbook};
use census_store::Store;

pub(in crate::restate_services) fn bind(
    store: &Store,
    request: BindRunRequest,
) -> Result<BindRunReply, JobError> {
    let run = requested_run(&request)?;
    let jurisdictions =
        super::requests::admitted_jurisdictions(&request.jurisdictions).map_err(|error| {
            JobError::Terminal {
                message: format!("{error:?}"),
            }
        })?;
    let identity: String = store
        .store_identity()
        .map_err(|error| JobError::Terminal {
            message: format!("store identity check failed: {error}"),
        })?
        .into();
    let manifest = if let Some(selection) = request.selection {
        let options = workbook::options_for_selected_request(
            store,
            &selection,
            &workbook::Options {
                grad_year: None,
                out: None,
                limit: None,
                scope: Scope::AllSources,
                jurisdictions: request.jurisdictions,
                school_year: run.season(),
            },
        )
        .map_err(JobError::from)?;
        let cohort =
            options
                .grad_year
                .and_then(GradYear::new)
                .ok_or_else(|| JobError::Terminal {
                    message: "selected run requires one checked graduation cohort".into(),
                })?;
        let manifest = RunManifest {
            store_identity: identity,
            run,
            cohort,
            jurisdictions,
        };
        store.bind_run(&manifest).map_err(|error| match error {
            census_store::StoreError::Refused { detail } => JobError::Terminal { message: detail },
            other => JobError::Terminal {
                message: format!("binding the run manifest failed: {other}"),
            },
        })?;
        manifest
    } else {
        let retained = store
            .run_manifest()
            .map_err(JobError::from)?
            .ok_or_else(|| JobError::Terminal {
                message: "current run binding requires an explicit selected profile".into(),
            })?;
        if retained.store_identity != identity
            || retained.run != run
            || retained.cohort != GradYear::CO2027
            || retained.jurisdictions != jurisdictions
        {
            return Err(JobError::Terminal {
                message: "historical run request differs from its retained binding".into(),
            });
        }
        retained
    };
    Ok(BindRunReply {
        store_identity: manifest.store_identity,
        season: request.season,
        revision: request.revision,
        cohort: manifest.cohort.get(),
        jurisdictions: manifest.jurisdictions,
    })
}

fn requested_run(request: &BindRunRequest) -> Result<CensusRun, JobError> {
    let season = SchoolYear::new(request.season).ok_or_else(|| JobError::Terminal {
        message: format!("invalid run school year {}", request.season),
    })?;
    CensusRun::new(season, request.revision).ok_or_else(|| JobError::Terminal {
        message: format!("invalid run revision {}", request.revision),
    })
}

#[cfg(test)]
mod refusal_tests;
#[cfg(test)]
mod seal_tests;
#[cfg(test)]
mod tests;
