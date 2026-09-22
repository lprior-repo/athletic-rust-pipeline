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

use super::policy::{
    artifact, fetched, CRAWL_DELAY_TEN_RPS, FETCHER_RPS, SCHOOL_COACH_CONTACT, SCHOOL_COACH_NAMES,
};
use super::SourceCapabilities as Caps;
use super::{SourceDescriptor, TransportKind};

/// Every adapter that fetches or parses external source material, in slug order.
///
/// Multiple adapters behind one origin each get their own entry, and each repeats the origin's
/// policy: that repetition is what makes a budget violation visible, because two entries for one host
/// can be compared against the one rate the host permits.
pub const REGISTRY: &[SourceDescriptor] = &[
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
        transport: TransportKind::StructuredApi,
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
        // records it as an `ObservedGrade` dated by the meet's school year. No meet_discovery: the
        // index that would enumerate meets sits behind the robots-disallowed `/api/`, so the `/raw`
        // URL is supplied by the operator. No athlete_profile: only the profile *URL* is retained;
        // that page is never fetched. Evidence is stamped `milesplit_<state>`
        // (`wire::Site::source_id`), not this slug.
        capabilities: Caps {
            athlete_discovery: true,
            bulk_results: true,
            grade_evidence: true,
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
        slug: "tfrrs",
        provider: "TFRRS high-school performance lists (state instances)",
        transport: TransportKind::Html,
        // One request returns one performance list (1.9 MB, 1,671 distinct athletes in the Indiana
        // sampler) and one more returns a team's roster; both URLs are supplied by the operator and
        // read through `ctx.fetcher.get` under the crate's default per-host spacing. The host is an
        // instance of the `<state>.tfrrs.org` family, so the policy is stated once for the family:
        // the adapter derives the jurisdiction from the host (`parse::jurisdiction_of_url`) and
        // refuses a page whose host names no state, rather than filing schools under a guessed one.
        // The host's `robots.txt` is comment-only, so the fetcher's default spacing applies
        // unchanged. Only the high-school instances serve this surface — `indiana` is the wired one;
        // `florida` and `nh` are the others. Evidence is stamped per state, `tfrrs_in`/`tfrrs_nh`
        // (`source_id` in the adapter root), the same per-instance naming `milesplit` uses.
        //
        // athlete_discovery: `parse::parse_list_page` returns one row per listed competitor
        // (`ParsedRow.athlete`) and `parse::parse_team_page` a team's roster
        // (`ParsedRoster.athletes`), and `map::Absorb::athlete_for` mints one `CanonicalAthlete` per
        // listed athlete. bulk_results: `map::Absorb::absorb_list` absorbs every section of one list
        // page, and each published mark becomes a `CanonicalPerformance`. grade_evidence: the row's
        // `Year` column is decoded by `map::row::grade_for` (`parse::YearToken::grade`; a token below
        // high school is refused, and a row that prints none falls back to the `?year=` the page was
        // requested with), then dated by the row's own meet date through
        // `parse::PublishedDate::school_year` and recorded as an `ObservedGrade`. school_evidence:
        // every row names the team route `/teams/tf/<School>_<gender>.html`, which
        // `map::Absorb::school_for` resolves against the consolidated index and mints when the index
        // has never seen the name. No meet_discovery: meets arrive inside result rows, not as an
        // index. No athlete_profile: lists and rosters only, never `/athletes/<id>`. No
        // graduation_evidence: the published token is a grade, not a cohort.
        capabilities: Caps {
            athlete_discovery: true,
            bulk_results: true,
            grade_evidence: true,
            school_evidence: true,
            ..Caps::NONE
        },
        admission: fetched("tfrrs.org", FETCHER_RPS),
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
