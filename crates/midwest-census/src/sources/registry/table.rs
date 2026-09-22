//! The registry table: one entry per adapter that fetches or parses external source material.
//!
//! A module is absent on purpose when it is machinery rather than a provider. `compiled`, `hytek`,
//! `raceday` and `xc` parse result-file bytes that `wiaa_results` fetched and dispatched to them
//! (`wiaa_results::parse`, `run_artifacts`), and `result_file` holds the shared shapes they return;
//! `mod.rs` holds the adapter context. None of them issues a request or names an origin, so an entry
//! here would declare a provider that does not exist.
//!
//! Evidence discipline: the comment above each entry names the symbol its `true` capabilities rest
//! on, in the adapter's own file, so a reviewer can check the claim without reading the adapter end
//! to end. A capability that cannot be pointed at is not claimed.

use super::SourceCapabilities as Caps;
use super::{SourceAdmission, SourceDescriptor, TransportKind};
use std::num::NonZeroUsize;

/// The spacing a source fetched through the shared fetcher is held to: `--delay-ms` defaults to
/// 1000 ms, which is half the collection's 2 rps ceiling.
const FETCHER_RPS: f64 = 1.0;

/// `www.wayzataresults.com` publishes `Crawl-delay: 10` for `User-agent: *`, and the fetcher applies
/// a robots crawl-delay as a floor on the configured spacing — so the host is declared at 0.1 rps
/// rather than at the default.
const CRAWL_DELAY_TEN_RPS: f64 = 0.1;

/// Origin recorded for an adapter that reads a checked-in research artifact: it issues no request, so
/// there is no host to pace and the declared ceiling is the one a live fetch of the same material
/// would inherit.
const ARTIFACT_ORIGIN: &str = "local-artifact";

/// The association-directory shape: a member-school universe, a coach and athletic-director
/// directory, and the professional address the directory publishes for those roles.
const SCHOOL_COACH_CONTACT: Caps = Caps {
    school_evidence: true,
    coach_directory: true,
    public_professional_contact: true,
    ..Caps::NONE
};

/// The name-only shape: a member-school universe plus coach and director names, with no address
/// layer at all, so no contact claim can leak out of a directory that publishes none.
const SCHOOL_COACH_NAMES: Caps = Caps {
    school_evidence: true,
    coach_directory: true,
    ..Caps::NONE
};

/// Admission of one origin fetched through the shared fetcher: `rps` spacing, one request in flight,
/// robots crawl-delay raised over the configured spacing and never lowered under it.
const fn fetched(origin: &'static str, rps: f64) -> SourceAdmission {
    SourceAdmission {
        origin,
        target_requests_per_second: rps,
        maximum_in_flight: NonZeroUsize::MIN,
        robots_crawl_delay_respected: true,
    }
}

/// Admission of an adapter that reads an artifact instead of contacting a host.
const fn artifact() -> SourceAdmission {
    fetched(ARTIFACT_ORIGIN, FETCHER_RPS)
}

/// Every adapter that fetches or parses external source material, in slug order.
///
/// Multiple adapters behind one origin each get their own entry, and each repeats the origin's
/// policy: that repetition is what makes a budget violation visible, because two entries for one host
/// can be compared against the one rate the host permits.
pub const REGISTRY: &[SourceDescriptor] = &[
    SourceDescriptor {
        slug: "athleticlive",
        provider: "AthleticLIVE meet harvest (research artifact)",
        transport: TransportKind::Csv,
        // meet_discovery: `meets::build_meets` mints one `CanonicalMeet` per harvested row, keyed on
        // the meet the row names. No bulk_results: the harvest carries a `has_results` flag, not the
        // result rows themselves. Nothing is requested by this adapter, so the admission states the
        // ceiling a live fetch of the same harvest would inherit. The evidence it writes carries the
        // caller's `source_label`, not this slug; the report's non-core list names the harvest
        // `athleticlive_meets_csv` (`report::NON_CORE_SOURCE_IDS`).
        capabilities: Caps {
            meet_discovery: true,
            ..Caps::NONE
        },
        admission: artifact(),
    },
    SourceDescriptor {
        slug: "athleticlive_athletes",
        provider: "AthleticLIVE athlete index (Elasticsearch)",
        transport: TransportKind::StructuredApi,
        // athlete_discovery: `ENDPOINT` (/athlete_list/_search) indexes one document per
        // athlete-entry at a meet, and `map::build_entities` mints `CanonicalAthlete`s from them.
        // grade_evidence: the query filters on the `y` keyword that `tokens::grade_from_token`
        // reads, dated through `GradYear::of`. school_evidence: `map::build_entities` mints the
        // school each row competes for.
        capabilities: Caps {
            athlete_discovery: true,
            grade_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("search.athletic.live", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "athleticnet",
        provider: "Athletic.net athlete bio API",
        transport: TransportKind::StructuredApi,
        // athlete_profile: `BIO_ENDPOINT` (GetAthleteBioData) answers one athlete per call, and
        // `collect` spends two calls per athlete because `sport=tf` and `sport=xc` return disjoint
        // results. No athlete_discovery: ids arrive from the operator's registry file, because the
        // endpoint that would discover them is robots-disallowed (`parse_targets`, `read_registry`).
        // grade_evidence: the bio's `grades` map, absorbed as `ObservedGrade`. school_evidence:
        // `map::school_for` mints the school each `allTeams` entry names.
        capabilities: Caps {
            athlete_profile: true,
            grade_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("www.athletic.net", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "coach_contacts",
        provider: "Researched official coach-contact dataset",
        transport: TransportKind::Csv,
        // school_evidence + coach_directory: `entities::row_entities` returns the school and the
        // coach/AD rows one dataset row names. public_professional_contact: the row's
        // `public_professional_email` column is copied onto `CanonicalCoach::professional_email`.
        // This adapter reads a checked-in dataset and issues no request, and it stamps its evidence
        // `coach_contacts_csv`, not this slug (`entities::row_entities`).
        capabilities: SCHOOL_COACH_CONTACT,
        admission: artifact(),
    },
    SourceDescriptor {
        slug: "ihsa",
        provider: "Illinois High School Association (IHSA) API",
        transport: TransportKind::StructuredApi,
        // school_evidence: `map::parse_school` mints a `CanonicalSchool` per `/v1/schools` row.
        // coach_directory: `map::parse_coach` with `staff::parse_coach_title`/`staff::parse_role`.
        // public_professional_contact: the staff payload carries a `HasEmail` flag and no address;
        // `collect` pays for the reveal (`/v1/schools/{id}/staff/{person}/email`), `parse::parse_email`
        // accepts the body only when it parses as an address, and the address lands on
        // `CanonicalCoach::professional_email`.
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("api.ihsa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "ks",
        provider: "Kansas State High School Activities Association (KSHSAA) directory",
        transport: TransportKind::StructuredApi,
        // school_evidence: `parse::parse_school` mints a `CanonicalSchool` per directory record.
        // coach_directory: `parse::parse_ad_coach` mints the athletic director that record names.
        // public_professional_contact: the record's `ADEmail`, which `parse::parse_ad_coach` puts on
        // `professional_email` only when it is non-empty; the AD row itself is dropped when
        // `ADName` is empty.
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("kshsaa-api.kshsaa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "milesplit",
        provider: "MileSplit state sites",
        transport: TransportKind::Html,
        // athlete_discovery: `parse::parse_team_index` enumerates teams and `parse::parse_roster`
        // the graded athletes in each (`wire::RosterAthlete::athlete_id`). graduation_evidence: the
        // roster's `column-grad-year` cell is parsed straight into `GradYear`. school_evidence:
        // `normalize::roster_entities` mints the school that owns the roster. No athlete_profile:
        // only the profile *URL* is retained; that page is never fetched. Evidence is stamped
        // `milesplit_<state>` (`wire::Site::source_id`), not this slug.
        capabilities: Caps {
            athlete_discovery: true,
            graduation_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        // Each state site is `<code>.milesplit.com`, a subdomain of the declared origin; the policy
        // is stated once because every site serves the same surface under the same robots rules.
        admission: fetched("milesplit.com", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "mshsl",
        provider: "Minnesota State High School League (MSHSL)",
        transport: TransportKind::Html,
        // school_evidence: `map::school_entities` mints a `CanonicalSchool` per listed school.
        // coach_directory: `map::ad_coaches` and `map::coach_entities` mint the directors and
        // per-team coaches. public_professional_contact: `text::decode_cfemail` decodes the address
        // the page publishes, and `map::accept_coach_email` keeps only school-domain ones.
        // The school views are HTML; the team and coach surfaces the walk also reads (`jsonapi` view
        // under `/jsonapi/views/`, coaches under `COACH_API_PREFIX`) are the site's own JSON.
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("www.mshsl.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "ohsaa",
        provider: "OHSAA myOHSAA officials portal",
        transport: TransportKind::Html,
        // school_evidence: `map::school_entities` mints a `CanonicalSchool` per search result.
        // coach_directory: `pages::parse_sports_table`/`pages::parse_coach_cell` read the sport
        // sections and `map::school_entities` mints the AD and the per-sport head coaches.
        // public_professional_contact: the `mailto:` hrefs those pages publish become
        // `professional_email`, accepted only when `parse::valid_email` takes them. Evidence is
        // stamped `ohsaa_portal` (`SOURCE_ID`), not this slug.
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("officials.myohsaa.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "plain_names",
        provider: "NDHSAA and NSAA member directories (North Dakota, Nebraska)",
        transport: TransportKind::Html,
        // school_evidence: `nd::parse_nd_school_page` and `nsaa_coaches::parse_nsaa_school` mint the
        // member schools. coach_directory: `nd_coaches::nd_sport_coaches` and
        // `nsaa_coaches::nsaa_coaches` mint the coach/AD names. No public_professional_contact:
        // neither provider publishes an address, so every entity it mints has an empty
        // `professional_email`. Evidence is stamped `ndhsaa`, `nsaa` or `plain_names` depending on
        // the half that produced it (`ND_ADAPTER_ID`, `NSAA_ADAPTER_ID`, `ADAPTER_ID`).
        capabilities: SCHOOL_COACH_NAMES,
        // The one admission states the NDHSAA host. The Nebraska half fetches
        // `secure.nsaahome.org` under the same 1 rps policy; a single-origin admission cannot name
        // both, and the second host is listed here so the declaration stays reviewable.
        admission: fetched("ndhsaa.com", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "wiaa",
        provider: "WIAA school directory (Wisconsin)",
        transport: TransportKind::Html,
        // school_evidence: `map::school_entities` mints a `CanonicalSchool` per directory page.
        // coach_directory: the page's administration and head-coach tables become `CanonicalCoach`s
        // through the same function. public_professional_contact: `primitives::decode_cfemail`
        // decodes the published address (a plain `mailto:` cell is accepted as well), and only a
        // value that parses as an address is kept. Evidence is stamped `wiaa_directory`
        // (`SOURCE_ID`), not this slug.
        capabilities: SCHOOL_COACH_CONTACT,
        admission: fetched("schools.wiaawi.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "wiaa_results",
        provider: "WIAA state result archive",
        transport: TransportKind::Html,
        // bulk_results: one archive artifact is a whole meet, and `map_rows::record_row` mints one
        // `CanonicalPerformance` per published row. meet_discovery: the archive is a per-season meet
        // index and `map::absorb` (through `map::meet_for`) mints the meet each artifact names.
        // grade_evidence: the timer reports' own year column, read by `map_rows::record_row` and
        // dated through `GradYear::of`.
        // No athlete_discovery: the athletes `record_row` mints are a by-product of result rows, not
        // an index the adapter was asked for. Artifacts are HTML or plain text; the newer PDF
        // releases are indexed but not parsed.
        capabilities: Caps {
            bulk_results: true,
            meet_discovery: true,
            grade_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("www.wiaawi.org", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "wayzata",
        provider: "Wayzata Results",
        transport: TransportKind::Html,
        // meet_discovery: `parse::schedule_rows` reads the provider's own published schedule and the
        // walk mints one core meet per competition row. Nothing else: the `/links/<slug>` pages that
        // hold the live results are retained as provider keys and deliberately never fetched.
        // Evidence is stamped `wayzata_schedule` (`ADAPTER_ID`), not this slug.
        capabilities: Caps {
            meet_discovery: true,
            ..Caps::NONE
        },
        admission: fetched("www.wayzataresults.com", CRAWL_DELAY_TEN_RPS),
    },
];
