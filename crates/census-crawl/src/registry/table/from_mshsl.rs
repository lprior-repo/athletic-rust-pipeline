//! The registry's entries from `mshsl` on, in the table's published order — the other half of the
//! positional cut described in `through_milesplit`.

use super::super::policy::{
    fetched, CRAWL_DELAY_TEN_RPS, FETCHER_RPS, SCHOOL_COACH_CONTACT, SCHOOL_COACH_NAMES,
};
use super::super::SourceCapabilities as Caps;
use super::super::{SourceDescriptor, TransportKind};

/// The adapters whose slugs sort from `mshsl` on.
pub(super) const FROM_MSHSL: [SourceDescriptor; 9] = [
    SourceDescriptor {
        slug: "mpa",
        provider: "Maine Principals' Association school directory",
        transport: TransportKind::Html,
        // school_evidence: `map::school_entities` mints one `CanonicalSchool` per entry
        // `pages::parse_directory` reads off the school list, keyed by the normalized name because
        // the list publishes no stable id of its own. coach_directory: the same call mints a
        // `CanonicalCoach` for each row `pages::parse_staff_table` returns, and `map::strip_honorific`
        // is what keeps a "Coach " lead-in out of the published name. No public_professional_contact:
        // the staff table's phone column is a school number, and no row publishes a coach address, so
        // every entity this adapter mints carries an empty `professional_email`.
        capabilities: SCHOOL_COACH_NAMES,
        admission: fetched("www.mpa.cc", FETCHER_RPS),
    },
    SourceDescriptor {
        slug: "mshsl",
        provider: "Minnesota State High School League (MSHSL)",
        transport: TransportKind::Html,
        // school_evidence: `map::school_entities` mints a `CanonicalSchool` per listed school.
        // coach_directory: `map::ad_coaches` and `map::coach_entities` mint the directors and
        // per-team coaches. public_professional_contact: `text::decode_cfemail` decodes the address
        // the page publishes, and `map::published_coach_email` keeps it unless it is malformed — a
        // consumer mailbox is kept too, as the coach's personal address.
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
        // public_professional_contact: the `mailto:` hrefs those pages publish are routed by
        // `CanonicalCoach::set_published_email`; malformed values are the only addresses refused.
        // Evidence is stamped
        // `ohsaa_portal` (`SOURCE_ID`), not this slug.
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
        slug: "riil",
        provider: "Rhode Island Interscholastic League directory",
        transport: TransportKind::Html,
        // school_evidence: `map::school_entities` mints one `CanonicalSchool` per school table the
        // league's single directory page publishes. coach_directory: the same call mints a
        // `CanonicalCoach` for each XC/TF row label `pages::parse_sport_label` recognises. No
        // public_professional_contact: the page publishes no coach or director address, so every
        // entity it mints carries an empty `professional_email`. The whole league is one page, so
        // the run costs one to two GETs regardless of how many schools it covers.
        capabilities: SCHOOL_COACH_NAMES,
        admission: fetched("riil.org", FETCHER_RPS),
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
        // decodes the published address (a plain `mailto:` cell is accepted as well), and any
        // well-formed value is routed by `CanonicalCoach::set_published_email`. Evidence is stamped
        // `wiaa_directory` (`SOURCE_ID`), not this slug.
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
