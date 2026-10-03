use anyhow::{anyhow, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
#[cfg(test)]
use census_service::restate_services::NationalReport;
use clap::Args;
use std::time::Duration;

const POLL: Duration = Duration::from_secs(10);

const PROGRESS_EVERY: u64 = 6;

#[derive(Args, Debug, Clone)]
#[command(
    about = "The flags every command that submits a run shares: which deployment to address, which revision of the run it belongs to, and how long to observe it"
)]
pub(super) struct WorkflowFlags {
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    pub(super) ingress: Option<String>,
    #[arg(
        help = "Run revision. A run that already exists is reattached to, and this is the deliberate way to invalidate completed work: the identity a run is addressed by carries the season, the run scope the parameters admit and this revision, so the same parameters reproduce their run while a changed jurisdiction set derives an identity of its own instead of re-attaching to another scope"
    )]
    #[arg(long, default_value_t = 1)]
    pub(super) revision: u32,
    #[arg(
        help = "Seconds to observe the run before returning. The run itself continues either way"
    )]
    #[arg(long, default_value_t = 172_800)]
    pub(super) timeout_seconds: u64,
}

impl WorkflowFlags {
    pub(super) fn rounds(&self) -> u64 {
        self.timeout_seconds
            .checked_div(POLL.as_secs())
            .map_or(0, |value| value)
    }
}

#[derive(Args, Debug, Clone)]
#[command(
    about = "The flags the workflow-driving commands share: [`WorkflowFlags`] plus the season, which together with the run scope and the revision is a national run's identity, and how the report prints"
)]
pub(super) struct RunFlags {
    #[command(flatten)]
    workflow: WorkflowFlags,
    #[arg(help = "Season start year: 2026 is the 2026-27 school year")]
    #[arg(long, default_value_t = 2026)]
    season: i16,
    #[arg(help = "Print the run's report as JSON instead of a table")]
    #[arg(long)]
    pub(super) json: bool,
}

impl RunFlags {
    fn season(&self) -> Result<SchoolYear> {
        named_season(self.season)
    }

    fn revision(&self) -> Revision {
        Revision(self.workflow.revision)
    }

    fn ingress(&self) -> Option<&str> {
        self.workflow.ingress.as_deref()
    }

    fn rounds(&self) -> u64 {
        self.workflow.rounds()
    }
}

fn named_season(year: i16) -> Result<SchoolYear> {
    SchoolYear::new(year).ok_or_else(|| {
        anyhow!(
            "--season {year} is not a school year ({}..={})",
            SchoolYear::MIN_START_YEAR,
            SchoolYear::MAX_START_YEAR
        )
    })
}

#[derive(Args, Debug, Clone)]
#[command(about = "`national`: the root run, fanned out over every jurisdiction")]
pub(super) struct NationalArgs {
    #[command(flatten)]
    flags: RunFlags,
    #[arg(
        help = "Jurisdictions to cover. Absent means the 48 contiguous states and the District of Columbia (49 jurisdictions)"
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(help = "Bypass cached bodies for this run")]
    #[arg(long)]
    refresh: bool,
    #[arg(
        help = "Rosters per jurisdiction; absent means every team the jurisdiction's index lists"
    )]
    #[arg(long)]
    limit_per_state: Option<usize>,
    #[arg(help = "Rosters one jurisdiction fetches concurrently")]
    #[arg(long, default_value_t = 4)]
    concurrency: usize,
    #[arg(help = "Submit and return. The run continues under Restate")]
    #[arg(long)]
    detach: bool,
}

#[derive(Args, Debug, Clone)]
#[command(
    about = "`jurisdiction`: one jurisdiction's census, for qualification and for retrying a single state"
)]
pub(super) struct JurisdictionArgs {
    #[command(flatten)]
    flags: RunFlags,
    #[arg(help = "The jurisdiction to census")]
    jurisdiction: UsJurisdiction,
    #[arg(help = "Bypass cached bodies for this run")]
    #[arg(long)]
    refresh: bool,
    #[arg(help = "Rosters to walk; absent means every team the jurisdiction's index lists")]
    #[arg(long)]
    limit_per_state: Option<usize>,
    #[arg(help = "Rosters fetched concurrently")]
    #[arg(long, default_value_t = 4)]
    concurrency: usize,
    #[arg(help = "Submit and return. The run continues under Restate")]
    #[arg(long)]
    detach: bool,
}

#[derive(Args, Debug, Clone)]
#[command(about = "`national-report`: the last report a national run wrote, without starting one")]
pub(super) struct NationalReportArgs {
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
    #[arg(help = "Season start year: 2026 is the 2026-27 school year")]
    #[arg(long, default_value_t = 2026)]
    season: i16,
    #[arg(help = "Run revision")]
    #[arg(long, default_value_t = 1)]
    revision: u32,
    #[arg(
        help = "Jurisdictions the run covered. Absent means the 48 contiguous states and the District of Columbia (49 jurisdictions)",
        long_help = "Jurisdictions the run covered. Absent means the 48 contiguous states and the District of Columbia (49 jurisdictions).\n\nThe set is part of the run's identity, so the report of a run submitted with `--states` is only reachable with the same list."
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(help = "Print the report as JSON")]
    #[arg(long)]
    json: bool,
}

mod observe;
mod report;
mod run;

pub(super) use observe::drive_jurisdiction;
#[cfg(test)]
use report::{blocked_count, failure_exit, owed_total};
pub(super) use run::{run_jurisdiction, run_national, run_national_report};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
