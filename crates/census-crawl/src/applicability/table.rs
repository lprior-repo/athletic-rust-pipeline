//! The applicability table itself: one row per registered adapter, in slug order.
//!
//! Sources, all under `research/midwest-source-program/`:
//!
//! * `data/source-coverage-matrix.csv` — 66 rows: the six `12-state` rollups, the twelve target
//!   states' five families each, and the `report_ref` column the citations here come from.
//! * `synthesis/05-source-matrix.md` §0a (the national lanes, which cover all 51 jurisdictions),
//!   §1a/§1b (the per-state rows, including the `unknown`/absence notations quoted below), §2 (what
//!   each family is for and where it stops) and §3b (rejected routes).
//! * `data/coach-contacts.csv` — the jurisdictions that have any contact row at all.
//! * the adapter's own module doc, where it states its provider's geography (`wayzata`, `tfrrs`,
//!   `plain_names`) or the host it reads.
//!
//! A row that cannot point at one of those does not belong here. `athleticlive` and
//! `athleticlive_athletes` are the two the registry declares as reading artifacts instead of
//! contacting a host; both are planned, because a plan wants the sources that cost no request.

use super::Applicability;
use census_domain::UsJurisdiction;

/// The rows: one per registered adapter, in slug order, each with the jurisdictions the research
/// evidences it for.
pub(super) const TABLE: [Applicability; 14] = [
    Applicability {
        slug: "athleticlive",
        jurisdictions: &[
            UsJurisdiction::Alabama,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Indiana,
            UsJurisdiction::Kansas,
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Missouri,
            UsJurisdiction::NorthDakota,
            UsJurisdiction::Nebraska,
            UsJurisdiction::Ohio,
            UsJurisdiction::Wisconsin,
        ],
        evidence: "`athleticlive + timers` rows for the twelve target states, `verified` in eleven of them: MN 6 blob \
                   requests -> 959 rows (242/242 Juniors at 2025 state XC) [10], ND 4 state XC races plus RTDB standings \
                   carrying `y` [25], MO 10 HTML files -> 3,393 rows [22], WI PrimeTime API 319 meets 2025 [08], IA \
                   per-event JSON + `search.athletic.live` ES [12]. AL (`xt.anet.live` via Xpress Timing) from §1b. The \
                   adapter reads harvest/capture artifacts and contacts no host (`admission.origin` = `local-artifact`).",
        refusal: "Hawaii is the one other jurisdiction with a §1b capture (`live.athletic.net/meets/58854`) and it is \
                  outside the census scope, so a run does not plan it; SD is `verified (absence)`: regular-season \
                  results are not published by any source, so its meets reach the graph through Athletic.net alone [26]. \
                  Every remaining jurisdiction has no AthleticLIVE harvest or capture in the corpus, so there is nothing \
                  for the adapter to read.",
    },
    Applicability {
        slug: "athleticlive_athletes",
        jurisdictions: &[
            UsJurisdiction::Alabama,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Indiana,
            UsJurisdiction::Kansas,
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Missouri,
            UsJurisdiction::NorthDakota,
            UsJurisdiction::Nebraska,
            UsJurisdiction::Ohio,
            UsJurisdiction::Wisconsin,
        ],
        evidence: "The adapter's own doc: `search.athletic.live/athlete_list/_search` indexes one document per \
                   athlete-entry at a meet and carries `y` (grade), `ani` (Athletic.net athlete id) and `t.ani` (team id), \
                   which makes it the second independent athlete source in the census. It is planned exactly where the \
                   platform's meet plane is evidenced — the `athleticlive` row — because a query is keyed on that \
                   platform's meet ids: MN 242 rows with `ani` at 2025 state XC [10], ND live-standings rows per athlete \
                   [25].",
        refusal: "Planned only where the platform's meet plane is evidenced (see the `athleticlive` row). With no meet of \
                  that state indexed by AthleticLIVE there is no id to query, and the census does not query by name.",
    },
    Applicability {
        slug: "athleticnet",
        jurisdictions: &UsJurisdiction::CENSUS_SCOPE,
        evidence: "`athletic_net_team_universe` [01], `_profile` [02], `_meet` [03], `_id_graph` [05] and `_rankings` \
                   [04], all `verified`, plus national lane §0a: `GetNavInfo` returns all 51 jurisdictions in one \
                   session-bound call (53 state nodes, 51 mapped) and `GetRankings` takes a server-side `grades:[11]` \
                   filter.",
        refusal: "No in-scope jurisdiction is excluded: the API answers a node per state, which is why this row is the \
                  whole census scope. Per-state *depth* is that state's own column in §1a/§1b — a state whose surface \
                  publishes no Athletic.net id still plans this source, at the depth its row states.",
    },
    Applicability {
        slug: "coach_contacts",
        jurisdictions: &[
            UsJurisdiction::Arizona,
            UsJurisdiction::California,
            UsJurisdiction::Colorado,
            UsJurisdiction::DistrictOfColumbia,
            UsJurisdiction::Florida,
            UsJurisdiction::Georgia,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Indiana,
            UsJurisdiction::Kansas,
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Missouri,
            UsJurisdiction::NorthDakota,
            UsJurisdiction::Nebraska,
            UsJurisdiction::NewJersey,
            UsJurisdiction::Ohio,
            UsJurisdiction::Oregon,
            UsJurisdiction::Tennessee,
            UsJurisdiction::Texas,
            UsJurisdiction::Utah,
            UsJurisdiction::Wisconsin,
        ],
        evidence: "`coach_contact_graph` [29], `verified (sampled; yields recorded per state)`: \
                   `data/coach-contacts.csv` holds 6,215 rows for exactly the jurisdictions listed (WI 3,166, KS 556, MN \
                   389, WI/IL 183 each, ...), each with the role, the published professional address where the provider \
                   publishes one, and the page it was observed on.",
        refusal: "`[coach-directories-national]` collected 22 tier-1 directories of 51: the rest are login-gated (MI \
                  `my.mhsaa.com`, MS 403 WAF), client-rendered (PA, NH, TX `/files/` robots-disallowed), or publish names \
                  with no address anywhere (ND 0 emails, SD 4 coach emails from a prior study), and a directory the \
                  adapter cannot read is not a planned source.",
    },
    Applicability {
        slug: "ihsa",
        jurisdictions: &[UsJurisdiction::Illinois],
        evidence: "[13] IHSA API: 828 member schools (801 full + 26 approved + 1 associate), `/staff2` + \
                   `/staff/<pid>/email` per-row email reveal (49/49 sampled rows HasEmail), census 125/125 Co2027 with \
                   coach email. Re-derivable from `evidence/gaps/38/il-schools.json`.",
        refusal: "Illinois only — the API is the association's own. T&F state finals only via \
                  `/v1/track-field/events/<id>/summary`; the XC qualifier list is 1,214 boys, 358 of grade 11.",
    },
    Applicability {
        slug: "ks",
        jurisdictions: &[UsJurisdiction::Kansas],
        evidence: "[23] KSHSAA directory API: 348 schools in `PublicClassifications` 2026, 526 directory records with AD \
                   name + email validated at 100% agreement, and `RetrieveResultsByActivity` (TF 2000-2026 = 20,887 rows, \
                   grade oracle at state).",
        refusal: "Kansas only. `/Shared/GetSchools` (940 rows) is a different id space and is never summed with \
                  `PublicClassifications`; the API serves KSHSAA members alone.",
    },
    Applicability {
        slug: "milesplit",
        jurisdictions: &UsJurisdiction::CENSUS_SCOPE,
        evidence: "`milesplit (<st>.milesplit.com)` rows for the twelve target states [07][27] — roster enumeration free, \
                   Co2027 enumerable, grad-year semantics verified — plus national lane §0a: 51/51 \
                   `<code>.milesplit.com/teams` hosts answered `200`, 26,562 HS teams, 7,500-URL rolling sitemap.",
        refusal: "No in-scope jurisdiction is excluded: every state host answered, including the states whose association \
                  publishes no result surface of its own (DE 82 team links, ME/RI partner links). The paywall boundary \
                  states per state in §1a/§1b what is free and what is not.",
    },
    Applicability {
        slug: "mshsl",
        jurisdictions: &[UsJurisdiction::Minnesota],
        evidence: "[09] MSHSL: 664 unique `/schools/<slug>` in sitemap page 1 -> 1,328 roster targets (664 x XC+TF); \
                   `/jsonapi/views/teams/list_school` and `/api/coaches/<nid>` carry head/assistant names and \
                   school-domain professional emails; `/api/team-data/schedule/<school>/<level>/<activity>` carries the \
                   Athletic.net MeetID.",
        refusal: "Minnesota only. Regular season is not published by MSHSL (it lives with the timers), so this source \
                  enumerates schools, teams and coaches — the meet plane is the `athleticlive` row's business.",
    },
    Applicability {
        slug: "ohsaa",
        jurisdictions: &[UsJurisdiction::Ohio],
        evidence: "[19] myOHSAA: 815 schools with distinct `OhsaaSchoolId` (100-1000027) re-derivable from \
                   `evidence/gaps/45/ohsaa-enrollment.html`; 3 requests/school (`SearchSchool` -> `SportsInformation` -> \
                   `AthleticDirector`), 56 sampled cells / 33 named with 100% of named cells carrying an email; census \
                   456/456 Co2027 with coach email.",
        refusal: "Ohio only. The portal is myOHSAA's own; `officials.myohsaa.org` is the third-party host it is read \
                  through, under the admission the descriptor declares.",
    },
    Applicability {
        slug: "plain_names",
        jurisdictions: &[UsJurisdiction::NorthDakota, UsJurisdiction::Nebraska],
        evidence: "[25] NDHSAA 169 schools (22-school coach sample; names only — no email field exists) and [24] NSAA 312 \
                   schools in one POST (1,528 staff rows / 1,217 names, validated at 100% wholesale, zero email fields \
                   anywhere); the adapter doc states the two name-only associations and that every entity it mints has \
                   `professional_email == None`.",
        refusal: "North Dakota and Nebraska only: the two member directories the adapter parses. Neither publishes an \
                  address, which is why the row carries the names shape and makes no contact claim.",
    },
    Applicability {
        slug: "tfrrs",
        jurisdictions: &[
            UsJurisdiction::Florida,
            UsJurisdiction::Indiana,
            UsJurisdiction::NewHampshire,
        ],
        evidence: "[28][18] `AUTHENTICATED-NOT (HS depth), PRIMARY only in IN`: Indiana HSR lists yield 1,861 Co2027 in \
                   2 requests (`&year=JR`; the `&year=SR` trap is the at-meet grade, Class of 2026), plus meet results \
                   with grade; §1b FL: TFRRS lists carry `Year` = FR/SO/JR/SR (4A Region 1: 1,469 rows); the adapter doc \
                   records the instances it reads (`indiana.tfrrs.org`, `nh.tfrrs.org`, `florida.tfrrs.org`).",
        refusal: "Every other jurisdiction: high-school depth is not anonymously reachable there ([28] WI `REJECT as \
                  result source`; index pages 0 HS-marked across MN/IA/MI/OH/MO/KS/NE/ND/SD) and the adapter refuses a \
                  host that names no state, so a national or sport instance is not a planned path either.",
    },
    Applicability {
        slug: "wayzata",
        jurisdictions: &[
            UsJurisdiction::Iowa,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Wisconsin,
        ],
        evidence: "The adapter doc names the provider a Minnesota / Iowa / Wisconsin timer publishing one \
                   server-rendered schedule table per sport and season; `[synthesis/10-measured-census.md]` ranks \
                   `wayzata` third among meet providers with 519 meet rows, and the meets it lists are core evidence (no \
                   Athletic.net surface is requested).",
        refusal: "The provider publishes schedules for its own three states. A meet outside them reaches the graph \
                  through the timer that published it, not through this adapter.",
    },
    Applicability {
        slug: "wiaa",
        jurisdictions: &[UsJurisdiction::Wisconsin],
        evidence: "[06] WIAA directory: 516 published schools; team-seasons with TeamIDs = 1,796 returned in 4 \
                   requests/season (462 boys TF / 462 girls TF / 437 boys XC / 435 girls XC); `GetDirectorySchool?orgID=` \
                   carries Superintendent, Principal, AD and per-sport head coaches with published emails (9-school \
                   sample: any TF/XC email 9/9).",
        refusal: "Wisconsin only. The universe is not re-derivable from retained bytes (1 of 26 directory letters was \
                  captured), so this row plans a live directory read, not a replay.",
    },
    Applicability {
        slug: "wiaa_results",
        jurisdictions: &[UsJurisdiction::Wisconsin],
        evidence: "[06][08] WIAA tournament file set: 104 result-file links/season across AN export, Hy-Tek, HTML and \
                   legacy formats; the adapter dispatches each body to the `compiled` / `hytek` / `raceday` / `xc` parsers \
                   and journals what it read.",
        refusal: "Wisconsin only — the archive is the association's own publication. A file dated outside the years a \
                  season may open in is refused rather than filed, so a mis-dated artifact cannot enter the graph.",
    },
];
