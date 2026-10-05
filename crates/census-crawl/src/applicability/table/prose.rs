pub(super) const AIA_EVIDENCE: &str = "`aia` AZ: aiaonline.org answers an all-paths-allowed robots and serves two public surfaces - \
                   `/schools/search.json?q=` (at most 10 rows per query; an empty query returned 20 of the ~287 members, so \
                   discovery is a city/name sample, not the whole inventory) and server-rendered school profiles `/schools/<id>` \
                   whose sport cards name one head coach per sport and school (e.g. `Cross Country - Boy's`, \
                   `Track & Field - Girl's`). The 2026-10-04 captures under `crates/census-crawl/tests/fixtures/aia/` hold the \
                   search JSON plus three profiles (Chandler 100, Seton 68, Hamilton 116), each with cross-country and track \
                   cards and a street address. Coach emails are not published; the admin directory requires a login.";

pub(super) const AIA_REFUSAL: &str = "One association's site: it lists Arizona member schools only, and no query returns the whole inventory, so a \
                  school a query does not match is not thereby absent from membership. Every other jurisdiction's association \
                  publishes on a different host.";

pub(super) const ATHLETICLIVE_EVIDENCE: &str = "`athleticlive + timers` rows for the twelve target states, `verified` in eleven of them: MN 6 blob \
                   requests -> 959 rows (242/242 Juniors at 2025 state XC) [10], ND 4 state XC races plus RTDB standings \
                   carrying `y` [25], MO 10 HTML files -> 3,393 rows [22], WI PrimeTime API 319 meets 2025 [08], IA \
                   per-event JSON + `search.athletic.live` ES [12]. AL (`xt.anet.live` via Xpress Timing) from §1b. The \
                   adapter reads harvest/capture artifacts and contacts no host (`admission.origin` = `local-artifact`).";

pub(super) const ATHLETICLIVE_REFUSAL: &str = "Hawaii is the one other jurisdiction with a §1b capture (`live.athletic.net/meets/58854`) and it is \
                  outside the census scope, so a run does not plan it; SD is `verified (absence)`: regular-season \
                  results are not published by any source, so its meets reach the graph through Athletic.net alone [26]. \
                  Every remaining jurisdiction has no AthleticLIVE harvest or capture in the corpus, so there is nothing \
                  for the adapter to read.";

pub(super) const NCES_EVIDENCE: &str = "The two national universes the census addresses are built from: the CCD school file \
                   `ccd_sch_029_2526_w_0a_050626.csv` (102,102 rows over 65 header-named columns inside \
                   `ccd_sch_029_2526.zip`, sha256 `b5d8dc34…`; every row carries `SCH_NAME`, a 12-digit `NCESSCH`, a \
                   mailing or location street, a five-digit ZIP and a two-letter state) and the PSS public-use file \
                   `pss2324_pu.csv` (22,510 rows over 359 columns, sha256 `14a2f9e6…`; every row carries `PPIN`, \
                   `PINST`, `PADDRS`, `PCITY`, `PSTABB`, `PZIP` and `LATITUDE24`/`LONGITUDE24`). Both readers key on \
                   header names and refuse a file that lacks them. The reader is an artifact reader \
                   (`admission.origin` = `local-artifact`): the operator supplies the extracted CSV, and the CCD ZIP is \
                   decompressed outside the process.";

pub(super) const NCES_REFUSAL: &str = "Alaska and Hawaii are outside the census scope, and the territories the files carry (PR, GU, VI, AS, \
                  MP) are not census jurisdictions, so those rows are skipped with their line and field recorded. An \
                  in-scope row without a usable name or a 12-digit NCES id is skipped the same way. Nothing else is \
                  excluded: a closed or future school stays in the corpus as the file publishes it.";

pub(super) const PA_PIAA_EVIDENCE: &str = "The PIAA member school directory at `www.piaa.org/schools/directory/list.aspx?alpha=<L>` renders \
                   one `<dl class=\"schoolBlock\">` per member school (id, name, printed address line) across the 24 \
                   linked letters (A..W and Y; X and Z print no link, and `alpha=Z` echoes the A group). Each school's \
                   details page (`details.aspx?ID=<id>`) adds the PIAA district, school district, school type and \
                   enrollment figures plus the superintendent, principal and athletic-director vCards. The adapter \
                   stores the schools with their city and reads each school's details page for the \
                   athletic-director posts, emitted as `CoachRole::AthleticDirector` rows with no sport and \
                   `Gender::Mixed`. Captures: `alpha=A` 53 schools, `alpha=B` 101 schools, details ID=12048 one \
                   athletic-director row with a published address.";

pub(super) const PA_PIAA_REFUSAL: &str =
    "Pennsylvania only. The PIAA directory is the state association's own publication; no other \
                   jurisdiction in the corpus uses this host or this URL shape.";
pub(super) const ATHLETICLIVE_ATHLETES_EVIDENCE: &str = "The adapter's own doc: `search.athletic.live/athlete_list/_search` indexes one document per \
                   athlete-entry at a meet and carries `y` (grade), `ani` (Athletic.net athlete id) and `t.ani` (team id), \
                   which makes it the second independent athlete source in the census. It is planned exactly where the \
                   platform's meet plane is evidenced — the `athleticlive` row — because a query is keyed on that \
                   platform's meet ids: MN 242 rows with `ani` at 2025 state XC [10], ND live-standings rows per athlete \
                   [25].";

pub(super) const ATHLETICLIVE_ATHLETES_REFUSAL: &str = "Planned only where the platform's meet plane is evidenced (see the `athleticlive` row). With no meet of \
                  that state indexed by AthleticLIVE there is no id to query, and the census does not query by name.";
pub(super) const ATHLETICNET_EVIDENCE: &str = "`athletic_net_team_universe` [01], `_profile` [02], `_meet` [03], `_id_graph` [05] and `_rankings` \
                   [04], all `verified`, plus national lane §0a: `GetNavInfo` returns all 51 jurisdictions in one \
                   session-bound call (53 state nodes, 51 mapped) and `GetRankings` takes a server-side `grades:[11]` \
                   filter.";

pub(super) const ATHLETICNET_REFUSAL: &str = "No in-scope jurisdiction is excluded: the API answers a node per state, which is why this row is the \
                  whole census scope. Per-state *depth* is that state's own column in §1a/§1b — a state whose surface \
                  publishes no Athletic.net id still plans this source, at the depth its row states.";

pub(super) const CIAC_EVIDENCE: &str = "`coach-directories-national` CT: the FusionPoint directory's 190 school tables carry 182 schools with \
                   at least one XC/indoor+outdoor-track Head Coach row (1,043 such rows) and 189 Athletic Director \
                   rows, counted over `samples/dir/CT__directory.html` by `tools/dir_coach_counts.py`. The lane \
                   records `PRIMARY - tier 1 names TF/XC head coaches per side` and notes the directory has no email \
                   column. Cost: 1 request.";

pub(super) const CIAC_REFUSAL: &str = "One league's directory: no other jurisdiction in the corpus publishes on this host, and the lane's \
                  rows for the rest name a different provider or none.";

pub(super) const CHSAA_EVIDENCE: &str = "378 member schools with embedded JSON at chsaanow.com/schools/ (schoolCode, name, officialName, city, streetAddress, zipCode); per-school coach JSON at chsaanow.com/schools/<slug>/ with activityName and positions[].title (Head Coach/Assistant Coach). 378/378 have streetAddress, 376/378 have zipCode. robots.txt allows / with only /history/champions/individual/totals/repeat/ and /preview/ disallowed.";

pub(super) const CHSAA_REFUSAL: &str =
    "One association's directory: no other jurisdiction in the corpus publishes on this host.";

pub(super) const COACH_DIRECTORIES_EVIDENCE: &str = "`research/sources/coach-directories-national/SOURCE_REPORT.md` DragonFly section, \
                   2026-09-29: 15 associations measured to publish staff via \
                   `maxinfosite-api-live.dragonflyathletics.com/states/<ruleset>/directory/` and \
                   `/schools/<shortCode>/summary`; 11,194 directory rows over 15 pages, 26,566 coach \
                   rows across the 15 jurisdictions. NC NCHSAA: 452 schools, 5,181 coaches; AL AHSAA: \
                   793 schools, 5,895 coaches; AR ArkAA: 521 schools, 3,585 coaches.";

pub(super) const COACH_DIRECTORIES_REFUSAL: &str = "The 34 other continental associations publish no staff on this platform; their \
                  directories are district-wide universes (1–73% of rows are elementary/middle schools) \
                  whose school identity `ccd_frame` already carries. They are not registered for this \
                  source on any page.";

pub(super) const COACH_CONTACTS_EVIDENCE: &str = "`coach_contact_graph` [29], `verified (sampled; yields recorded per state)`: \
                   the 2026-09-21 research-workspace artifact held 6,215 rows for exactly the jurisdictions listed (WI 3,166, \
                   KS 556, MN 389, WI/IL 183 each, ...), each with the role, the published professional address where the \
                   provider publishes one, and the page it was observed on.";

pub(super) const COACH_CONTACTS_REFUSAL: &str = "`[coach-directories-national]` collected 22 tier-1 directories of 51: the rest are login-gated (MI \
                  `my.mhsaa.com`, MS 403 WAF), client-rendered (PA, NH, TX `/files/` robots-disallowed), or publish names \
                  with no address anywhere (ND 0 emails, SD 4 coach emails from a prior study), and a directory the \
                  adapter cannot read is not a planned source.";

pub(super) const HOME_CAMPUS_EVIDENCE: &str = "Home Campus \
                   (`www.cifsshome.org`) serves CA CIF sections 1-9 and 13 (1,727 school buttons across the ten sections in the \
                   2026-10-04 probe), FL FHSAA section 10 (880 buttons, 230,370 B) and NJ NJSIAA section 12 (452 buttons, \
                   133,880 B) from `widget/school/directory?section=<n>`; each button carries a numeric school id. \
                   `widget/get-school-details/<id>/details` answers 200 application/json only with an XHR context \
                   (`X-Requested-With: XMLHttpRequest` plus a same-origin `Referer`; 403 without) and publishes \
                   `coaches[] = {firstname, lastname, sport, sport_id, level_name, aft_name, email}` plus \
                   `athleticFaculties[] = {firstname, lastname, aft_name, email, work_phone}`; Arcadia (19) 24 coaches / \
                   6 faculty, Bolles (1872) 26 / 8, Abraham Clark (3374) 0 / 0. Captures live under \
                   `crates/census-crawl/tests/fixtures/home_campus/` and \
                   `research/sources/coach-coverage-bundle-20261004/probes/home_campus/`; robots is the 24-byte \
                   `Disallow:`-empty form and collection paces at 0.5 requests/second per host: a solo \
                   2026-10-04 run passed 1,101 requests at 1 rps, two concurrent runs at roughly twice \
                   that rate drew a `Human Verification` challenge, and the halved rate keeps two \
                   uncoordinated runs at the rate the host tolerated.";

pub(super) const HOME_CAMPUS_REFUSAL: &str = "One vendor portal: it covers only the sections it serves (CIF 1-9 and 13, \
                  FHSAA 10, NJSIAA 12); no other association publishes here and other states need their own host. The \
                  JSON carries one row per sport and level and often no role, so athlete-facing rosters, assistant \
                  coverage beyond the listed rows and coach phone numbers are not established by this source.";

pub(super) const SIDEARM_STAFF_EVIDENCE: &str = "The verified gomats.org SIDEARM staff directory \
                   publishes Miramonte High School in window.client_title and article.sidearm-staff \
                   with header-bound sport, level, title and name cells. Same-row firstHalf/secondHalf \
                   literals yield Brian Henderson's Cross Country email, Robert Kennedy's Track & Field \
                   email and Sean Hennessy's exact Athletic Director contact. The 2026-10-04 capture is \
                   305,587 bytes; crates/census-crawl/tests/fixtures/sidearm_staff/SOURCE.md records its \
                   digest and robots provenance. The registry admission enforces Crawl-delay: 30.";

pub(super) const SIDEARM_STAFF_REFUSAL: &str = "Only gomats.org and its California school are verified; \
                  SIDEARM platform membership is not a national host inventory. School state is contextual \
                  CA from the published MaxPreps link, not a verified address; no city is inferred. Other \
                  hosts, custom-column variants and email mechanisms are unverified. Only published XC/TF \
                  coaches and the exact Athletic Director title become canonical contacts.";

pub(super) const IHSAA_EVIDENCE: &str = "[13] IHSA API: 828 member schools (801 full + 26 approved + 1 associate), `/staff2` + \
                   `/staff/<pid>/email` per-row email reveal (49/49 sampled rows HasEmail), census 125/125 Co2027 with \
                   coach email. Re-derivable from `evidence/gaps/38/il-schools.json`.";

pub(super) const IHSAA_REFUSAL: &str = "Illinois only — the API is the association's own. T&F state finals only via \
                  `/v1/track-field/events/<id>/summary`; the XC qualifier list is 1,214 boys, 358 of grade 11.";

pub(super) const KSAA_EVIDENCE: &str = "[23] KSHSAA directory API: 348 schools in `PublicClassifications` 2026, 526 directory records with AD \
                   name + email validated at 100% agreement, and `RetrieveResultsByActivity` (TF 2000-2026 = 20,887 rows, \
                   grade oracle at state).";

pub(super) const KSAA_REFUSAL: &str = "Kansas only. `/Shared/GetSchools` (940 rows) is a different id space and is never summed with \
                  `PublicClassifications`; the API serves KSHSAA members alone.";

pub(super) const MILESPPLIT_EVIDENCE: &str = "`milesplit (<st>.milesplit.com)` rows for the twelve target states [07][27] — roster enumeration free, \
                   Co2027 enumerable, grad-year semantics verified — plus national lane §0a: 51/51 \
                   `<code>.milesplit.com/teams` hosts answered `200`, 26,562 HS teams, 7,500-URL rolling sitemap.";

pub(super) const MILESPPLIT_REFUSAL: &str = "No in-scope jurisdiction is excluded: every state host answered, including the states whose association \
                  publishes no result surface of its own (DE 82 team links, ME/RI partner links). The paywall boundary \
                  states per state in §1a/§1b what is free and what is not.";

pub(super) const MPA_EVIDENCE: &str = "`coach-directories-national` ME: the same FusionPoint directory shape, 147 school tables carrying 81 \
                   schools with an XC/indoor+outdoor-track Head Coach row (310 such rows) and 149 Athletic Director \
                   rows, counted by `tools/dir_coach_counts.py`. The lane records `PRIMARY` and notes the directory \
                   has no email column. Cost: 1 request.";

pub(super) const MPA_REFUSAL: &str = "One association's directory: the rest of the census scope is served by other providers, by ones the \
                  lane found unworkable, or by none.";

pub(super) const MSHSL_EVIDENCE: &str = "[09] MSHSL: 664 unique `/schools/<slug>` in sitemap page 1 -> 1,328 roster targets (664 x XC+TF); \
                   `/jsonapi/views/teams/list_school` and `/api/coaches/<nid>` carry head/assistant names and \
                   published emails, including consumer mailboxes; `/api/team-data/schedule/<school>/<level>/<activity>` \
                   carries the Athletic.net MeetID.";

pub(super) const MSHSL_REFUSAL: &str = "Minnesota only. Regular season is not published by MSHSL (it lives with the timers), so this source \
                  enumerates schools, teams and coaches — the meet plane is the `athleticlive` row's business.";

pub(super) const OHSA_EVIDENCE: &str = "[19] myOHSAA: 815 schools with distinct `OhsaaSchoolId` (100-1000027) re-derivable from \
                   `evidence/gaps/45/ohsaa-enrollment.html`; 3 requests/school (`SearchSchool` -> `SportsInformation` -> \
                   `AthleticDirector`), 56 sampled cells / 33 named with 100% of named cells carrying an email; census \
                   456/456 Co2027 with coach email.";

pub(super) const OHSA_REFUSAL: &str = "Ohio only. The portal is myOHSAA's own; `officials.myohsaa.org` is the third-party host it is read \
                  through, under the admission the descriptor declares.";

pub(super) const PLAIN_NAMES_EVIDENCE: &str = "[25] NDHSAA 169 schools (22-school coach sample; names only — no email field exists) and [24] NSAA 312 \
                   schools in one POST (1,528 staff rows / 1,217 names, validated at 100% wholesale, zero email fields \
                   anywhere); the adapter doc states the two name-only associations and that every entity it mints has \
                   `professional_email == None`.";

pub(super) const PLAIN_NAMES_REFUSAL: &str = "North Dakota and Nebraska only: the two member directories the adapter parses. Neither publishes an \
                  address, which is why the row carries the names shape and makes no contact claim.";

pub(super) const RIIL_EVIDENCE: &str = "`coach-directories-national` RI: 55 school tables carrying 49 schools with an XC/indoor+outdoor-track \
                   Head Coach row (259 such rows), 57 Athletic Director rows and 0 `mailto`, counted by \
                   `tools/dir_coach_counts.py`. The lane records `PRIMARY` and a 1-2 request cost.";

pub(super) const RIIL_REFUSAL: &str = "One league's directory: no other jurisdiction's lane row names this host, so a plan outside Rhode \
                  Island has nothing here to read.";

pub(super) const TFRS_EVIDENCE: &str = "[28][18] `AUTHENTICATED-NOT (HS depth), PRIMARY only in IN`: Indiana HSR lists yield 1,861 Co2027 in \
                   2 requests (`&year=JR`; the `&year=SR` trap is the at-meet grade, Class of 2026), plus meet results \
                   with grade; §1b FL: TFRRS lists carry `Year` = FR/SO/JR/SR (4A Region 1: 1,469 rows); the adapter doc \
                   records the instances it reads (`indiana.tfrrs.org`, `nh.tfrrs.org`, `florida.tfrrs.org`).";

pub(super) const TFRS_REFUSAL: &str = "Every other jurisdiction: high-school depth is not anonymously reachable there ([28] WI `REJECT as \
                  result source`; index pages 0 HS-marked across MN/IA/MI/OH/MO/KS/NE/ND/SD) and the adapter refuses a \
                  host that names no state, so a national or sport instance is not a planned path either.";

pub(super) const WAYZATA_EVIDENCE: &str = "The adapter doc names the provider a Minnesota / Iowa / Wisconsin timer publishing one \
                   server-rendered schedule table per sport and season; `[synthesis/10-measured-census.md]` ranks \
                   `wayzata` third among meet providers with 519 meet rows, and the meets it lists are core evidence (no \
                   Athletic.net surface is requested).";

pub(super) const WAYZATA_REFUSAL: &str = "The provider publishes schedules for its own three states. A meet outside them reaches the graph \
                  through the timer that published it, not through this adapter.";

pub(super) const WIAA_EVIDENCE: &str = "[06] WIAA directory: 516 published schools; team-seasons with TeamIDs = 1,796 returned in 4 \
                   requests/season (462 boys TF / 462 girls TF / 437 boys XC / 435 girls XC); `GetDirectorySchool?orgID=` \
                   carries Superintendent, Principal, AD and per-sport head coaches with published emails (9-school \
                   sample: any TF/XC email 9/9).";

pub(super) const WIAA_REFUSAL: &str = "Wisconsin only. The universe is not re-derivable from retained bytes (1 of 26 directory letters was \
                  captured), so this row plans a live directory read, not a replay.";

pub(super) const WIAA_RESULTS_EVIDENCE: &str = "[06][08] WIAA tournament file set: 104 result-file links/season across AN export, Hy-Tek, HTML and \
                   legacy formats; the adapter dispatches each body to the `compiled` / `hytek` / `raceday` / `xc` parsers \
                   and journals what it read.";

pub(super) const WIAA_RESULTS_REFUSAL: &str = "Wisconsin only — the archive is the association's own publication. A file dated outside the years a \
                  season may open in is refused rather than filed, so a mis-dated artifact cannot enter the graph.";

pub(super) const STATE_ED_EVIDENCE: &str = "New York's state-agency directory is the one captured: `data.nysed.gov/lists.php?type=school` \
                   (786,227 bytes, sha256 `adf44bb6…`) is an A-Z anchor index with 2,528 `profile.php?instid=` links, and \
                   `profile.php?instid=800000038718` (60,020 bytes) is the profile page carrying the address the index \
                   lacks. The reader is an artifact reader (`admission.origin` = `local-artifact`).";

pub(super) const STATE_ED_REFUSAL: &str = "Only New York has a state-agency capture; the prototype's CA/TX/NY/FL/PA URL guesses returned nothing \
                  and no other state artifact exists in the corpus, so no other jurisdiction is planned. The tabular \
                  index page is parsed with the same reader as the association directories; only the per-profile pages \
                  are state-agency specific. New Jersey, the prototype's sole live URL, returned nothing, so NJ is not \
                  planned; the reader accepts any operator-supplied file.";

pub(super) const TSSAA_EVIDENCE: &str = "`portal.tssaa.org/common/directory/?type=high` (41,350 bytes) lists the member schools and \
                   `?id=<n>` carries the per-school staff cards, captured for 456 schools (e.g. `id=3`, 79,409 bytes, \
                   sha256 `c69f3c2d…`); the prototype's Tennessee run counts 2,769 coach rows and 384 schools with a \
                   Track/CrossCountry row. `portal.tssaa.org/robots.txt` allows `/common` and disallows the rest, so \
                   this row plans the one path the host permits.";

pub(super) const TSSAA_REFUSAL: &str = "One association's directory: no other jurisdiction publishes on this host, and every path outside \
                  `/common` is robots-disallowed.";

pub(super) const UHSAA_EVIDENCE: &str = "`uhsaa.org/school-directory-new/` lists the member schools (324 anchors over 162 distinct school ids in the 2026-10-04 capture; the adapter parses 162 links) and \
                   `school-directory/?id=<Name>&Reg=<Region>&schoolID=<ID>` carries each school's published name, street address, \
                   district, classification and region plus one head coach per sport with a `mailto:` address (e.g. Alta `schoolID=1`: \
                   Rebecca Bennion, XC and track; Murray `schoolID=73`: Diana Stewart XC and JennaBree Tollestrup track; Herriman \
                   `schoolID=43`: Josh Pugel XC and Corey Wales track). `uhsaa.org/robots.txt` answers 200 with no relevant \
                   disallow, and the captures live under `crates/census-crawl/tests/fixtures/uhsaa/` plus \
                   `research/sources/uhsaa/`. Schools publish one head coach per sport and no assistant coaches, and no coach phone.";

pub(super) const UHSAA_REFUSAL: &str = "One association's site: it lists Utah member schools only, and every other jurisdiction's association \
                  publishes on a different host. Directory and profile paths are the only coach-coverage surfaces the host serves.";
