use super::{BootstrapError, ServeCaptureProtocol};
use crate::outcome::{DrainState, Outcome};
use crate::restate_services::CaptureExecution;
use crate::spawn::Spawner;
use census_store::Store;
use std::path::PathBuf;
use std::sync::Arc;

#[tracing::instrument(skip_all)]
pub(super) async fn open(
    region: &Spawner,
    data_dir: PathBuf,
    protocol: Option<ServeCaptureProtocol>,
) -> Result<(Arc<Store>, CaptureExecution), BootstrapError> {
    let Some(ServeCaptureProtocol::FreshQualificationV3) = protocol else {
        return Err(BootstrapError::CaptureProtocolUnsupported {
            raw: "(none)".to_string(),
        });
    };
    let outcome = region
        .blocking(move || {
            Store::open_current_qualification_v3(&data_dir)
                .map(capture_parts)
                .map_err(|source| store_open_error(data_dir, source))
        })
        .await;
    match outcome {
        Outcome::Ok(store) => Ok(store),
        Outcome::Err(e) => Err(e),
        Outcome::Panicked => Err(BootstrapError::StoreTask {
            state: DrainState::Panicked,
            reason: "task panicked".to_string(),
        }),
        Outcome::Cancelled => Err(BootstrapError::StoreTask {
            state: DrainState::Cancelled,
            reason: "task cancelled".to_string(),
        }),
    }
}

fn capture_parts(
    admitted: census_store::CaptureQualificationStore,
) -> (Arc<Store>, CaptureExecution) {
    let (store, writer) = admitted.into_parts();
    (
        Arc::new(store),
        CaptureExecution::FreshQualification(Arc::new(writer)),
    )
}

fn store_open_error(path: PathBuf, source: census_store::StoreError) -> BootstrapError {
    match source {
        census_store::StoreError::Installation(
            census_store::InstallationError::EnrollmentOnlyUnsupported {
                store_identity,
                config_id,
            },
        ) => BootstrapError::EnrollmentOnlyUnsupported {
            store_identity,
            config_id,
        },
        source => BootstrapError::StoreOpen {
            path,
            source: Box::new(source),
        },
    }
}
