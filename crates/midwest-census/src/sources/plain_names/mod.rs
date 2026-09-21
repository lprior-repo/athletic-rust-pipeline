//! NDHSAA + NSAA adapter (North Dakota, Nebraska): school universes and coach **names**.
//!
//! Two name-only state associations share this file because their contract is identical: a canonical
//! school per member school plus coach/athletic-director rows with **no email layer at all**. The
//! value here is the school universe and coach-name attribution, not contact addresses, so every
//! entity written by this adapter has `professional_email == None` and the report says so.
//!
//! # Requests (no browser, no Athletic.net, both server-rendered)
//!
//! * **North Dakota** — `GET https://ndhsaa.com/schools` lists all 169 member schools, then one
//!   `GET https://ndhsaa.com/schools/<id>/<slug>` per school carries the staff block
//!   (Superintendent, Principal, Athletic/Activities Director …) and the
//!   `Sport/Activity Offering | Coaches` table.
//! * **Nebraska** — `GET https://secure.nsaahome.org/nsaaforms/direxportscreen.php` returns the
//!   request form plus the `<option>` list of all **312** member schools, then one
//!   `GET …?session=&school=<name>` per school returns its full record: Superintendent, Principal,
//!   Athletics/Activities Director(s), and one row per sport. NSAA publishes no numeric school id in
//!   any surface, so the published school name *is* the provider key space.
//!
//!   The screen also offers a bulk form (`session= `, `school=View all schools`,
//!   `submit=See School Info`) that renders all 312 schools into one 1,085,584-byte body — but it
//!   needs **49.4 s** to render (measured HTTP 200 2026-09-20T14:34:47Z), past the fetcher's 45 s
//!   client timeout, so that single response is never usable in this harness. A per-school request
//!   answers in ~0.3 s and journals per school, which is what the walk below does.
//!
//! # Role policy (hard rule)
//!
//! * Office/building staff are never emitted as coaches or athletic directors: the token list in
//!   `OFFICE_ROLE_TOKENS` drops secretaries, business managers, technology directors, trainers,
//!   principals and superintendents **before** any director/coach match is attempted, so an athletic
//!   director's secretary and a superintendent are both unpresent regardless of seniority.
//! * NDHSAA's table is headed "Coaches" and never says which name is the head coach, so sport-scoped
//!   North Dakota rows carry [`CoachRole::Unknown`] rather than an invented head/assistant split.
//!   Nebraska publishes exactly one coach per sport, so its sport rows are [`CoachRole::HeadCoach`].
//! * A multi-name cell ("Trey Schlueter/Katie Winters") is split into one entity per person — the
//!   alternative is storing two humans inside one `name` field.
//!
//! # Deliberately never read
//!
//! Phone numbers, fax numbers and street addresses. The staff-line regex stops at the first `<`, so
//! `<p>Phone: <a href="tel:…">…</a></p>` can never become a name, and the only school metadata this
//! adapter stores are city, enrolment and website. No athlete data exists in either source.
//!
//! # Resume
//!
//! Phases `ndhsaa_schools` / `ndhsaa_coaches` / `nsaa_schools` / `nsaa_coaches`, journal key
//! `<state>:<provider-key>` (`ND:1045`, `NE:Adams Central`). Both halves check the journal
//! **before** the per-school GET, so an interrupted walk re-fetches nothing; Nebraska's form GET is
//! served from the fetcher's disk cache on re-runs, and `options.limit` caps how many schools a
//! smoke run processes without disturbing the journal of a full walk.

use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use anyhow::Result;

// The module doc links `CoachRole`; rustdoc needs it in scope, rustc does not.
#[cfg(doc)]
use census_domain::model::CoachRole;

/// NDHSAA member-school index (server-rendered HTML, 169 schools, no pagination).
pub const ND_SCHOOLS_URL: &str = "https://ndhsaa.com/schools";
/// NDHSAA per-school page prefix; a school page is `<prefix><id>/<slug>`.
pub const ND_SCHOOL_BASE: &str = "https://ndhsaa.com/schools/";
/// NSAA school-directory export screen. A bare GET returns the request form **plus the option list
/// of all 312 member schools**; `?session=&school=<name>` returns one school's full record.
///
/// The screen's own bulk form (`session= `, `school=View all schools`, `submit=See School Info`)
/// renders all 312 schools into one 1,085,584-byte response, but takes **49.4 s** to do it
/// (measured 2026-09-20T14:34:47Z, HTTP 200) while the fetcher's client timeout is 45 s — the crate
/// therefore never receives that body. The per-school route answers in ~0.3 s and journals per
/// school, so it is what this adapter walks.
pub const NSAA_FORM_URL: &str = "https://secure.nsaahome.org/nsaaforms/direxportscreen.php";

/// Adapter id recorded in evidence for the North Dakota half.
const ND_ADAPTER_ID: &str = "ndhsaa";
/// Adapter id recorded in evidence for the Nebraska half.
const NSAA_ADAPTER_ID: &str = "nsaa";
/// Adapter id for the combined report.
const ADAPTER_ID: &str = "plain_names";

const ND_SCHOOLS_PHASE: &str = "ndhsaa_schools";
const ND_COACHES_PHASE: &str = "ndhsaa_coaches";
const NSAA_SCHOOLS_PHASE: &str = "nsaa_schools";
const NSAA_COACHES_PHASE: &str = "nsaa_coaches";

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Stop after this many schools (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached HTTP bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
    pub observed_on: String,
    /// Restrict to these state codes when the provider spans several states.
    pub states: Vec<String>,
    /// School names to resolve when the provider has no bulk index.
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

// The moved test module reaches these three through `use super::*`; no production path in this
// file needs them, so they are bound for tests only instead of widening their visibility.
#[cfg(test)]
use nsaa::NSAA_ALL_SCHOOLS;
#[cfg(test)]
use parse::email_regex;
#[cfg(test)]
use parse::split_person_names;

// ---------------------------------------------------------------------------------------------------
// Shared context helpers
// ---------------------------------------------------------------------------------------------------

/// Evidence date: `options.observed_on` when set, else the run's own date.
fn observed_on(ctx: &AdapterContext<'_>, options: &Options) -> String {
    match nonempty(&options.observed_on) {
        Some(date) => date,
        None => ctx.observed_on.clone(),
    }
}

/// Fetch options for this adapter: the context's options, plus `options.refresh` when set.
fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    let mut fetch = ctx.fetch_options();
    fetch.refresh = options.refresh || ctx.refresh;
    fetch
}

// ---------------------------------------------------------------------------------------------------
// Collect
// ---------------------------------------------------------------------------------------------------

/// Collect this provider's schools and coach/AD contacts into the canonical store.
///
/// `options.states` selects the half to run: `ND` the North Dakota half, `NE` the Nebraska half, and
/// an empty list both. Each half is independent, journalled separately and honours `options.limit`.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER_ID, "schools");
    let before = ctx.fetcher.stats().await;
    let fetch = fetch_options(ctx, options);

    let states: Vec<String> = options
        .states
        .iter()
        .map(|state| state.trim().to_ascii_uppercase())
        .filter(|state| !state.is_empty())
        .collect();
    let run_nd = states.is_empty() || states.iter().any(|state| state == "ND");
    let run_ne = states.is_empty() || states.iter().any(|state| state == "NE");

    // Counters saturate: they only feed the report, and no source carries 2^64 rows.
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
        report.note(format!(
            "no provider selected: states {:?} match neither ND nor NE",
            options.states
        ));
    }
    if !options.school_names.is_empty() {
        report.note("school_names ignored: both providers publish a bulk school index");
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.rows = schools_written;
    report.note(format!(
        "wrote {schools_written} schools and {coaches_written} coach/AD rows"
    ));
    // Neither provider publishes a coach email; every coach entity is written with `None`.
    report.with_email = 0;
    report.note("names only: provider publishes no coach email");
    Ok(report)
}

#[cfg(test)]
mod tests;
