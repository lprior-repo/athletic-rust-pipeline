//! The source capability registry: what each adapter can be asked for, and what a request to it
//! costs its origin.
//!
//! Why this module exists: a plan picks sources *before* any adapter runs, and that knowledge lived
//! in prose spread across the adapter module docs. The objective's §10/§11 answer is one
//! machine-readable declaration per adapter — the capabilities a request buys and the per-origin
//! admission it must respect — so a plan can spend its request budget on the shapes that pay
//! (ADR-004: one whole-meet payload beats N per-athlete profile calls), and so a declaration that is
//! faster than the transport enforces is visible in review instead of in traffic.
//!
//! Constraints this table obeys:
//!
//! * Every `true` capability is evidenced in the adapter's own file, and the symbol it rests on is
//!   named in the comment above that descriptor. A capability that cannot be pointed at in code is
//!   `false`: an over-claimed registry is worse than an incomplete one, because a planner believes
//!   it.
//! * `target_requests_per_second` never exceeds the collection's 2 rps per-host ceiling (the
//!   fetcher's `MIN_AUTHORIZED_DELAY` floor). It states the spacing a run actually applies: one
//!   request per second is `--delay-ms`'s default, and an origin whose `robots.txt` asks for more
//!   gets less. An adapter that reads a checked-in artifact issues no request at all, which its
//!   admission states through its origin rather than by claiming a host it never contacts.
//! * `maximum_in_flight` repeats the transport's own bound — one in-flight request per host — so
//!   widening it has to be a visible edit here rather than a silent one in a caller.
//! * `slug` is the adapter's module name, because that is what a plan names. Where an adapter stamps
//!   a different id on the evidence it writes (`ohsaa_portal` for `ohsaa`, `wiaa_directory` for
//!   `wiaa`), the entry says so: the slug is how the source is addressed, the id is what the store
//!   will contain.
//!
//! The table itself is in `table`; this file holds the vocabulary and the two queries a planner
//! calls.

mod policy;
mod table;

use std::num::NonZeroUsize;

pub use table::REGISTRY;

/// How a source's bytes arrive. The planner uses this to decide what a request buys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    /// A JSON API whose request carries parameters or a body and whose response is one document
    /// per call (`api.ihsa.org`, Athletic.net's bio endpoint, AthleticLIVE's search endpoint).
    StructuredApi,
    /// A JSON document fetched from a fixed path, with the request carried in the URL.
    StaticJson,
    /// A comma-separated artifact: a research harvest's row shape, read from a file.
    Csv,
    /// An XML document.
    Xml,
    /// A spreadsheet artifact.
    Xlsx,
    /// A server-rendered HTML page.
    Html,
    /// A PDF release.
    Pdf,
    /// A surface that only renders under a browser session.
    Browser,
}

/// What a source can be asked for, one flag per shape the planner weighs.
///
/// A `true` is a claim about the adapter's own code, not about the provider's website: the flags
/// describe what a registered request returns today, because that is all a plan can budget for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCapabilities {
    /// Enumerates athletes as its own index: a roster or directory whose rows are the competitors it
    /// lists. An adapter that mints athletes as a by-product of rows it was asked for something else
    /// (result rows, a team's schedule) does not claim this.
    pub athlete_discovery: bool,
    /// Answers one named athlete with that athlete's own record (a profile/bio call).
    pub athlete_profile: bool,
    /// Enumerates meets as its own index: a calendar, archive or harvest whose rows are the meets a
    /// plan can then acquire. A meet named inside one athlete's own record is not discovery.
    pub meet_discovery: bool,
    /// One payload carries many performers' results — a whole meet, not one athlete's profile.
    pub bulk_results: bool,
    /// Publishes an in-school grade (freshman..senior) dated to a meet or season.
    pub grade_evidence: bool,
    /// Publishes a graduating year directly, rather than a grade that still needs a date.
    pub graduation_evidence: bool,
    /// Publishes school identity: a member-school universe, per-school facts, or the school name
    /// and provider id a row attributes to an athlete.
    pub school_evidence: bool,
    /// Publishes a coach/athletic-director directory with the roles it names.
    pub coach_directory: bool,
    /// Publishes professional (school or association) addresses for those directory roles.
    pub public_professional_contact: bool,
    /// Publishes a personal-record claim of its own. No registered adapter reads such a field, so
    /// the flag is `false` across the table: a source that publishes results is not thereby a source
    /// that publishes a PR, and the platform keeps source-reported PRs distinct from the best marks
    /// it reduces itself (`crate::bests`).
    pub pr_evidence: bool,
}

impl SourceCapabilities {
    /// Every capability off. Descriptors spread this base, so a capability added to the set stays
    /// `false` for every source until that adapter's file is read and its evidence named — the safe
    /// direction for a table a planner trusts.
    const NONE: SourceCapabilities = SourceCapabilities {
        athlete_discovery: false,
        athlete_profile: false,
        meet_discovery: false,
        bulk_results: false,
        grade_evidence: false,
        graduation_evidence: false,
        school_evidence: false,
        coach_directory: false,
        public_professional_contact: false,
        pr_evidence: false,
    };
}

/// Per-origin admission the transport must respect (§10): the fetch layer already enforces one
/// in-flight request per host at or below 2 rps with robots crawl-delay; this declares what the
/// source *permits*, so a mismatch between declaration and enforcement is reviewable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceAdmission {
    /// The host the policy is stated for, e.g. `www.athletic.net`, or the placeholder an adapter
    /// that contacts no host declares instead.
    pub origin: &'static str,
    /// Spacing this origin is held to, in requests per second: what the run applies, never more than
    /// the collection's 2 rps ceiling. Lower than the configured rate when the origin's
    /// `robots.txt` publishes a `Crawl-delay`.
    pub target_requests_per_second: f64,
    /// Concurrent requests this origin permits. One is what the fetcher grants a host.
    pub maximum_in_flight: NonZeroUsize,
    /// Whether a published `Crawl-delay` is applied as a floor on spacing. Every entry here says
    /// `true`: the fetcher raises spacing to the origin's crawl-delay and never lowers it.
    pub robots_crawl_delay_respected: bool,
}

/// One adapter's declaration: what it is, how its bytes arrive, what a request buys, and what its
/// origin permits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceDescriptor {
    /// The adapter's module name under `sources/`, e.g. `wiaa_results`.
    pub slug: &'static str,
    /// Human provider name, e.g. `WIAA state result archive`.
    pub provider: &'static str,
    pub transport: TransportKind,
    pub capabilities: SourceCapabilities,
    pub admission: SourceAdmission,
}

/// The registered descriptor for one adapter module, or `None` when the slug names no source.
///
/// A linear scan of the static table: no allocation, and no second index to keep in step with
/// [`REGISTRY`].
pub fn descriptor(slug: &str) -> Option<&'static SourceDescriptor> {
    REGISTRY.iter().find(|entry| entry.slug == slug)
}

/// The planning order for a candidate list (§11, ADR-004): payloads that carry whole meets first,
/// then sources that enumerate meets, then the sources that name athletes (roster discovery or a
/// per-athlete profile), then the directories that cost a request without returning a performance.
///
/// Order inside a band is by slug, so the same set of slugs always yields the same plan whatever
/// order the caller listed it in. A slug that names no descriptor, or that repeats, contributes
/// nothing: a caller validates slugs with [`descriptor`] first, and letting one source appear twice
/// would give it two places in a request budget.
pub fn bulk_first(slugs: &[&str]) -> Vec<&'static SourceDescriptor> {
    let mut ordered: Vec<&'static SourceDescriptor> =
        slugs.iter().copied().filter_map(descriptor).collect();
    ordered.sort_by_key(|entry| (planning_band(&entry.capabilities), entry.slug));
    ordered.dedup_by_key(|entry| entry.slug);
    ordered
}

/// The band a capability set sorts into: bulk results, meet discovery, athlete-shaped, the rest.
///
/// The bands are ADR-004's preference order, so the ordering a planner applies and the bands a
/// capability report shows come from one place.
fn planning_band(capabilities: &SourceCapabilities) -> u8 {
    if capabilities.bulk_results {
        0
    } else if capabilities.meet_discovery {
        1
    } else if capabilities.athlete_discovery || capabilities.athlete_profile {
        2
    } else {
        3
    }
}

// The test module last, as every adapter does: the scan lane reads the production region as the
// lines before `#[cfg(test)]`, so code after this point would drop out of its counts.
#[cfg(test)]
#[path = "registry/tests.rs"]
mod tests;
