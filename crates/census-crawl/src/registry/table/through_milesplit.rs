//! The registry's entries up to and including `milesplit`, in the table's published order.
//!
//! `descriptors()` is one flat list, so the two shards here are a positional cut and not a taxonomy:
//! a new adapter belongs in the shard whose slugs bracket it, and the splice in `mod.rs` keeps the
//! published order either way. Each entry's comment names the symbol its `true` capabilities rest
//! on, which is what makes a reviewer's check possible without reading the adapter end to end.

use super::super::policy::{
    artifact, fetched, FETCHER_RPS, SCHOOL_COACH_CONTACT, SCHOOL_COACH_NAMES,
};
use super::super::SourceCapabilities as Caps;
use super::super::{SourceDescriptor, TransportKind};

/// The adapters whose slugs sort up to `milesplit`.
pub(super) const THROUGH_MILESPLIT: [SourceDescriptor; 8] = [
    SourceDescriptor {
        slug: "athleticlive",
        provider: "AthleticLIVE meet harvest and result-plane captures (research artifacts)",
        transport: TransportKind::Csv,
        // meet_discovery: `meets::build_meets` mints one `CanonicalMeet` per harvested row, keyed on
        // the meet the row names. bulk_results: the result-plane entry point `results::collect`
        // (re-exported as `collect_results`) folds operator-supplied captures of the three wire
        // routes and issues no request of its own; `results::absorb::absorb_document` mints the
        // event a document publishes and writes one `CanonicalPerformance` per row through
        // `map_rows::record_row`, while `results::absorb::absorb_standings` folds the live-standings
        // payload of a run whose document is missing through `map_rows::record_standing`.
        // grade_evidence:
        // `map_rows::read_grade` reads the row's `y` token and dates it to the meet's school year,
        // recording an `ObservedGrade` on the performance. No athlete_discovery: the harvest carries
        // no roster and the result rows only name athletes the document lists, which the vocabulary
        // counts as a by-product. No school_evidence: `map_rows::resolve_school` resolves a published
        // label against the consolidated school index and counts a miss, so this adapter publishes no
        // school identity of its own — the boundary `wiaa_results` holds too. Nothing is requested by
        // this adapter, so the admission states the ceiling a live fetch of the same harvest would
        // inherit. The evidence it writes carries the caller's `source_label`, not this slug, and the
        // result plane stamps `athleticlive_results`; the report's non-core list names both
        // (`report::NON_CORE_SOURCE_IDS`). `transport` names the harvest the slug's own `collect`
        // reads: the result-plane captures are JSON documents read from operator-supplied paths, so
        // the artifact admission covers both and no route is ever fetched.
        capabilities: Caps {
            meet_discovery: true,
            bulk_results: true,
            grade_evidence: true,
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
        provider: "Athletic.net athlete bio API and whole-meet pull",
        // The source policy's own lane (AGENTS.md, Source policy): Athletic.net is acquired through
        // the headed persistent-profile browser lane, with a `HumanRequired` handoff when a challenge
        // appears. The addresses are the ones the adapter names; what the browser changes is who
        // carries them — the one process that owns the profile, whose session a challenge belongs to
        // rather than to a fresh HTTP request the run cannot clear. A browser-transported source is
        // also what makes the plan's `BrowserSession` class reachable: `access_class` reads this
        // field, and a run with no lane configured defers the source instead of failing it.
        transport: TransportKind::Browser,
        // athlete_profile: `BIO_ENDPOINT` (GetAthleteBioData) answers one athlete per call, and
        // `collect` spends two calls per athlete because `sport=tf` and `sport=xc` return disjoint
        // results. bulk_results: one meet is a 2-request pull (`meet::meet_requests`), and
        // `meet::map::absorb_meet` (called from `meet::collect`) stores one `CanonicalPerformance`
        // per published row and relay leg through `meet::store::store`; the route is chosen when
        // `Options::meets` names ids, and nothing here enumerates meets. No athlete_discovery: ids
        // arrive from the operator's registry file or the harvest's meet links, because the endpoint
        // that would discover them is robots-disallowed (`parse_targets`, `read_registry`), and the
        // athletes a meet pull mints are a by-product of result rows. grade_evidence: the bio's
        // `grades` map and the meet rows' published grade, both absorbed as `ObservedGrade`
        // (`meet::read::grade_of` reads `9`..=`12` and refuses anything else). school_evidence:
        // `map::school_for` mints the school each `allTeams` entry names, and
        // `meet::map::absorb_meet` resolves each row's school the same way.
        capabilities: Caps {
            athlete_profile: true,
            bulk_results: true,
            grade_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("www.athletic.net", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "ciac",
        provider: "CIAC sports directory (Connecticut)",
        transport: TransportKind::Html,
        // school_evidence: `map::school_entities` mints one `CanonicalSchool` per
        // `DirectoryStaffTable` the directory page publishes. coach_directory: the same call mints a
        // `CanonicalCoach` for every row `pages::parse_sport_label` recognises, and
        // `pages::is_placeholder_name` is what keeps the rows the page fills with "TBA" or "Vacant"
        // out of the workbook. No public_professional_contact: the directory publishes no coach or
        // director address, so every entity it mints carries an empty `professional_email`.
        capabilities: SCHOOL_COACH_NAMES,
        admission: fetched("ciacsports.com", FETCHER_RPS),
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
        // `normalize::roster_entities` mints the school that owns the roster. bulk_results: one
        // `/raw` request returns a whole result set (`results::collect` ->
        // `fetch::fetch_result_set` -> `raw::parse_raw`), and `map_rows::record_row` mints one
        // `CanonicalPerformance` per published row. grade_evidence also arrives on that route:
        // `raw_rows::columns::grade_of` reads the row's `Yr` cell and `map_rows::record_athlete`
        // records it as an `ObservedGrade` dated by the meet's school year. meet_discovery: the
        // state results index is an allowed HTML page publishing fifty meets per response
        // (`parse::parse_meet_index`, `wire::Site::results_url`), and `census::collect_state_meets`
        // walks it page by page — journaled per page, ending on the index's own repeat signal — into
        // `source_meets` rows. From a row, one results-page request lists every result file the meet
        // has (`parse::parse_meet_result_files`, `wire::MeetResultFile::raw_url`), so the `/raw` URLs
        // are derived rather than supplied. The note this replaced said no meet discovery was
        // possible because the list sat behind `/api/`; the page it rested on publishes the list in
        // its own HTML (`tests/fixtures/milesplit/oh_meet_770621_results.html:356`). No
        // athlete_profile: only the profile *URL* is retained; that page is never fetched. Evidence
        // is stamped `milesplit_<state>`
        // (`wire::Site::source_id`), not this slug.
        capabilities: Caps {
            athlete_discovery: true,
            bulk_results: true,
            grade_evidence: true,
            graduation_evidence: true,
            meet_discovery: true,
            school_evidence: true,
            ..Caps::NONE
        },
        // Each state site is `<code>.milesplit.com`, a subdomain of the declared origin; the policy
        // is stated once because every site serves the same surface under the same robots rules.
        admission: fetched("milesplit.com", FETCHER_RPS),
    },
];
