use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};

#[cfg(doc)]
use census_domain::model::CoachRole;
use census_domain::UsJurisdiction;

pub const ND_SCHOOLS_URL: &str = "https://ndhsaa.com/schools";
pub const ND_SCHOOL_BASE: &str = "https://ndhsaa.com/schools/";
pub const NSAA_FORM_URL: &str = "https://secure.nsaahome.org/nsaaforms/direxportscreen.php";

const ND_ADAPTER_ID: &str = "ndhsaa";
const NSAA_ADAPTER_ID: &str = "nsaa";
const ADAPTER_ID: &str = "plain_names";

const ND_SCHOOLS_PHASE: &str = "ndhsaa_schools";
const ND_COACHES_PHASE: &str = "ndhsaa_coaches";
const NSAA_SCHOOLS_PHASE: &str = "nsaa_schools";
const NSAA_COACHES_PHASE: &str = "nsaa_coaches";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

mod nd;
mod nd_coaches;
mod nd_walk;
mod nsaa;
mod nsaa_coaches;
mod nsaa_walk;
mod parse;

pub use nd::{
    parse_nd_offerings, parse_nd_school_page, parse_nd_school_refs, parse_nd_staff, NdOffering,
    NdSchoolRef, NdStaffRole,
};
pub use nd_coaches::{nd_ad_coaches, nd_sport_coaches, parse_nd_role, parse_nd_sport};
pub use nsaa::{
    nsaa_school_url, parse_nsaa_directory, parse_nsaa_school_names, NsaaRole, NsaaRow, NsaaSchool,
};
pub use nsaa_coaches::{nsaa_coaches, parse_nsaa_row, parse_nsaa_school};

use nd_coaches::collect_north_dakota;
use nsaa_coaches::collect_nebraska;
use parse::nonempty;

#[cfg(test)]
use nsaa::NSAA_ALL_SCHOOLS;
#[cfg(test)]
use parse::email_regex;
#[cfg(test)]
use parse::split_person_names;

fn observed_on(ctx: &AdapterContext<'_>, options: &Options) -> String {
    match nonempty(&options.observed_on) {
        Some(date) => date,
        None => ctx.observed_on.clone(),
    }
}

fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    let mut fetch = ctx.fetch_options();
    fetch.refresh = options.refresh || ctx.refresh;
    fetch
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER_ID, "schools");
    let before = ctx.fetcher.stats().await;
    let fetch = fetch_options(ctx, options);

    let run_nd = options.states.is_empty() || options.states.contains(&UsJurisdiction::NorthDakota);
    let run_ne = options.states.is_empty() || options.states.contains(&UsJurisdiction::Nebraska);

    let mut schools_written = 0u64;
    let mut coaches_written = 0u64;
    if run_nd {
        let (schools, coaches) = collect_north_dakota(ctx, options, &fetch, &mut report).await?;
        schools_written = schools_written.saturating_add(schools);
        coaches_written = coaches_written.saturating_add(coaches);
    }
    if run_ne {
        let (schools, coaches) = collect_nebraska(ctx, options, &fetch, &mut report).await?;
        schools_written = schools_written.saturating_add(schools);
        coaches_written = coaches_written.saturating_add(coaches);
    }
    if !run_nd && !run_ne {
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "no provider selected: states {codes:?} match neither ND nor NE"
        ));
    }
    if !options.school_names.is_empty() {
        report.note("school_names ignored: both providers publish a bulk school index");
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.rows = schools_written;
    report.note(format!(
        "wrote {schools_written} schools and {coaches_written} coach/AD rows"
    ));
    report.with_email = 0;
    report.note("names only: provider publishes no coach email");
    Ok(report)
}

#[cfg(test)]
mod tests;
