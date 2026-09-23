//! The workflow-driving subcommands: `national`, `jurisdiction` and `national-report`.
//!
//! A census run does not belong to the shell that starts it. These commands submit durable work to
//! the local Restate server and observe it: killing one cancels nothing, and rerunning the same
//! command reattaches to the invocation already in flight. The run is identified by its workflow
//! identity (`national:<season>:<revision>`, `jurisdiction:<state>:<season>:<revision>`), so a retry
//! reuses the logical job instead of manufacturing a second one (§8).
//!
//! None of them opens the Fjall store. The census runs inside `midwest-serve`, which holds that lock
//! for the life of the process, so a command that opened the store would refuse the very run it is
//! asking for; `cli::run` therefore dispatches these before it opens anything.

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use clap::Args;
use midwest_census::census::Revision;
#[cfg(test)]
use midwest_census::restate_services::NationalReport;
use std::time::Duration;

/// How long a waiting command sleeps between two observations of the run it submitted.
const POLL: Duration = Duration::from_secs(10);

/// Polls between two progress lines: one minute at [`POLL`].
const PROGRESS_EVERY: u64 = 6;

/// The flags every command that submits a run shares: which deployment to address, which revision of
/// the run it belongs to, and how long to observe it.
#[derive(Args, Debug, Clone)]
pub(super) struct WorkflowFlags {
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    pub(super) ingress: Option<String>,
    /// Run revision. A run's identity is `<kind>:<season>:<revision>` — a run that already exists is
    /// reattached to, and changing a run *parameter* needs the revision bumped instead of a second
    /// submission under the same identity.
    #[arg(long, default_value_t = 1)]
    pub(super) revision: u32,
    /// Seconds to observe the run before returning. The run itself continues either way.
    #[arg(long, default_value_t = 172_800)]
    pub(super) timeout_seconds: u64,
}

impl WorkflowFlags {
    /// Observation rounds the timeout buys. A zero timeout means "submit and do not observe".
    pub(super) fn rounds(&self) -> u64 {
        self.timeout_seconds
            .checked_div(POLL.as_secs())
            .unwrap_or(0)
    }
}

/// The flags the workflow-driving commands share: [`WorkflowFlags`] plus the season, which together
/// with the revision is the run's identity, and how the report prints.
#[derive(Args, Debug, Clone)]
pub(super) struct RunFlags {
    #[command(flatten)]
    workflow: WorkflowFlags,
    /// Season start year: 2026 is the 2026-27 school year.
    #[arg(long, default_value_t = 2026)]
    season: i16,
    /// Print the run's report as JSON instead of a table.
    #[arg(long)]
    pub(super) json: bool,
}

impl RunFlags {
    fn season(&self) -> SchoolYear {
        SchoolYear(self.season)
    }

    fn revision(&self) -> Revision {
        Revision(self.workflow.revision)
    }

    /// The `--ingress` value as given: `None` means the local deployment.
    fn ingress(&self) -> Option<&str> {
        self.workflow.ingress.as_deref()
    }

    /// Observation rounds the timeout buys. A zero timeout means "submit and do not observe".
    fn rounds(&self) -> u64 {
        self.workflow.rounds()
    }
}

/// `national`: the root run, fanned out over every jurisdiction.
#[derive(Args, Debug, Clone)]
pub(super) struct NationalArgs {
    #[command(flatten)]
    flags: RunFlags,
    /// Jurisdictions to cover. Absent means all fifty states and the District of Columbia.
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    /// Bypass cached bodies for this run.
    #[arg(long)]
    refresh: bool,
    /// Rosters per jurisdiction; absent means every team the jurisdiction's index lists.
    #[arg(long)]
    limit_per_state: Option<usize>,
    /// Rosters one jurisdiction fetches concurrently.
    #[arg(long, default_value_t = 4)]
    concurrency: usize,
    /// Submit and return. The run continues under Restate.
    #[arg(long)]
    detach: bool,
}

/// `jurisdiction`: one jurisdiction's census, for qualification and for retrying a single state.
#[derive(Args, Debug, Clone)]
pub(super) struct JurisdictionArgs {
    #[command(flatten)]
    flags: RunFlags,
    /// The jurisdiction to census.
    jurisdiction: UsJurisdiction,
    /// Bypass cached bodies for this run.
    #[arg(long)]
    refresh: bool,
    /// Rosters to walk; absent means every team the jurisdiction's index lists.
    #[arg(long)]
    limit_per_state: Option<usize>,
    /// Rosters fetched concurrently.
    #[arg(long, default_value_t = 4)]
    concurrency: usize,
    /// Submit and return. The run continues under Restate.
    #[arg(long)]
    detach: bool,
}

/// `national-report`: the last report a national run wrote, without starting one.
#[derive(Args, Debug, Clone)]
pub(super) struct NationalReportArgs {
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
    /// Season start year: 2026 is the 2026-27 school year.
    #[arg(long, default_value_t = 2026)]
    season: i16,
    /// Run revision.
    #[arg(long, default_value_t = 1)]
    revision: u32,
    /// Print the report as JSON.
    #[arg(long)]
    json: bool,
}

mod observe;
mod report;
mod run;

pub(super) use observe::drive_jurisdiction;
#[cfg(test)]
use report::{blocked_count, cell, failure_exit, owed_total};
pub(super) use run::{run_jurisdiction, run_national, run_national_report};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
