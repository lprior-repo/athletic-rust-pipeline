
use std::time::Duration;

use restate_sdk::prelude::*;

pub const MAX_ROWS_PER_REQUEST: usize = 50_000;
pub const MAX_SWEEP_WINDOWS: u32 = 366;

pub const MAX_SWEEP_ENDPOINTS: usize = 256;

pub const MAX_LIMIT_PER_STATE: usize = 10_000;

const CENSUS_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60 * 60);

const CENSUS_ABORT_TIMEOUT: Duration = Duration::from_secs(60 * 60);

fn census_service_options() -> ServiceOptions {
    ServiceOptions::new()
        .inactivity_timeout(CENSUS_INACTIVITY_TIMEOUT)
        .abort_timeout(CENSUS_ABORT_TIMEOUT)
}

pub(super) fn census_service<D: IntoServiceDefinition>(definition: D) -> ServiceDefinition {
    definition
        .into_service_definition()
        .options(census_service_options())
}
