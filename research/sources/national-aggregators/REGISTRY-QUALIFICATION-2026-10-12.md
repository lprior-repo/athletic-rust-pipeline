# Registry / durable-arm qualification inventory - 2026-10-12

Lane: `research/sources/national-aggregators` (agent `FlashRegistryAudit`). Staged section for
`research/sources/national-aggregators/SOURCE_REPORT.md` (staged rather than spliced: that file is
49.6 KB and this lane's write tool replaces whole files).

Assignment: inventory every registered/applicable provider family against the six durable arms
(teams, rosters, meets, results, profile, contacts) and against the jurisdictions that make each
family applicable; mark every cell reachable-handler / outstanding-obligation / refusal; name where a
silent default treats an unsupported stage as complete; reconcile the F07/F11/F12 denominators named
in `docs/NATIONAL-CENSUS-PLAN.md` against what the arms actually dispatch.

## Method and limits

Read-only source audit at the working-tree revision of 2026-10-12. **No build, no test, no binary
was run** (the read-only assignment forbids it, and concurrent writers are active in
`census-service`/`census-report`). Every `path:line` below was confirmed by reading or by an exact
line-number search in this session; where a line range ends in a bare number it means "from that
line" (the symbol's extent was not fully enumerated). Statements derived from unexecuted source are
the audit's normal currency here; the places where a conclusion follows from several premises rather
than one line are marked `[INFERENCE chain]` with all premises named, so a reviewer can confirm each
by running the named test or handler.

Files read in full or by anchored ranges (the evidence base):

- `crates/census-crawl/src/registry/table/{mod.rs,through_milesplit.rs,from_mshsl.rs,directories.rs}`,
  `crates/census-crawl/src/registry/{policy.rs,tests.rs}`
- `crates/census-crawl/src/applicability.rs`, `crates/census-crawl/src/applicability/table/data.rs`,
  `crates/census-crawl/src/applicability/tests.rs`
- `crates/census-service/src/restate_services/{plan.rs,plan/strategies.rs,wire/plan.rs,jobs.rs,
  teams_arms.rs,teams_arms/known_sites.rs,meets_arms.rs,meets_arms/{walks.rs,prerequisites.rs},
  results_arms.rs,results_arms/{selection.rs,outcomes.rs,collection.rs,rows.rs,prerequisites.rs},
  open_work.rs,open_work/{history.rs,inventory.rs,store_evidence.rs},
  jurisdiction/{team_collection.rs,pipeline.rs,pipeline/history.rs,report.rs,stage_runs.rs,
  team_source.rs},national/{contact_phase.rs,coverage.rs},census.rs,census/requests.rs}`
- `crates/census-service/src/restate_services/wire/coach_lanes.rs`,
  `wire/coach_lanes/{missing.rs,sd.rs,ut.rs,registry_tests.rs}`
- `crates/census-service/src/census/{meets.rs,sweep.rs}`,
  `census/state/{open.rs,evidence.rs,evidence/recorded.rs,seal_digest.rs,seal/workbook.rs}`,
  `census/seal/assembly.rs`
- `crates/census-domain/src/jurisdiction/{table.rs,codes.rs}`
- `crates/census-report/src/...` (via the national coverage writer chain; not re-read line-exact)
- `docs/NATIONAL-CENSUS-PLAN.md` s8 F-table `:349-364`, `research/sources/national-aggregators/`
  (`SOURCE_REPORT.md` head, `TFRRS-DAT-PATH-QUALIFICATION-2026-10-12.md`)

## 1. Denominators

| Denominator | Value | Locator |
|---|---|---|
| Run scope: jurisdictions in `CENSUS_SCOPE` | **49** (48 contiguous states + DC) | `crates/census-domain/src/jurisdiction/table.rs` (`CENSUS_SCOPE`, `EXCLUDED_FROM_CENSUS` pin AK/HI); `applicability/tests.rs:89-105` asserts excluded jurisdictions plan nothing |
| Registered source families (`descriptors()`) | **29** = 12 + 10 + 7 | `registry/table/mod.rs:8-16`; `through_milesplit.rs` (`[SourceDescriptor; 12]`), `from_mshsl.rs` (`[SourceDescriptor; 10]`), `directories.rs` (`[SourceDescriptor; 7]`) |
| Applicability rows | **29** (`[Applicability; 29]`, ARBITER last) | `applicability/table/data.rs:1,251`; `applicability/table.rs:4-16` (ARBITER) |
| Registry <-> applicability identity | exact, tested | `registry/tests.rs:7-38` (`PLAN_SLUGS: [&str; 29]`), `:40-63` (registered exactly once), `:235` (every plan slug resolves), `:375` (every family has a source module); `applicability/tests.rs:5-13` (every row names a registered slug), `:16-26` (the table names every registered source once) |
| Family x jurisdiction pairs | **242** = 3 x 49 universal (milesplit, athleticnet, nces) + 95 scoped | derived from `applicability/table/data.rs` rows (each row's jurisdiction slice is quoted in §3) |
| Durable stages on a jurisdiction object | 6 named by `owing()`: teams, rosters, meets, results, contacts, publication | `census/state/open.rs:12-15` (`refused_sources`, `owed_rosters`, `owed_results`), `:34-53` (`owing()`, `refused_sources` named at `:51`) |
| Teams arms | **21** entries (20 registry slugs + `ihsa_tournament`, which is not a registry slug) | `restate_services/teams_arms.rs:20-42` |
| Meets arms | **4** | `restate_services/meets_arms.rs:19-24` |
| Results arms | **4** | `restate_services/results_arms.rs:35-40` |
| Coach-lane arms (contacts) | **21** `LANES` + **28** `missing::LANES` = 49 | `wire/coach_lanes.rs:53` (LANES, ends before `:215`); `wire/coach_lanes/missing.rs:18-187` |

## 2. Arm tables, verbatim

`TEAMS_ARMS` (`crates/census-service/src/restate_services/teams_arms.rs:20-42`):

```rust
pub(super) const TEAMS_ARMS: &[(&str, TeamsArm)] = &[
    (census::SOURCE, TeamsArm::MilesplitIndex),
    ("wiaa", TeamsArm::WiaaDirectory),
    ("mshsl", TeamsArm::MshslSchools),
    ("plain_names", TeamsArm::PlainNamesDirectories),
    ("ihsa", TeamsArm::IhsaSchools),
    ("ihsa_tournament", TeamsArm::IhsaTournament),
    ("ks", TeamsArm::KsDirectory),
    ("coach_directories", TeamsArm::CoachDirectories),
    ("arbiter_orgs", TeamsArm::ArbiterOrgs),
    ("ohsaa", TeamsArm::OhsaaSchools),
    ("mpa", TeamsArm::MpaSchools),
    ("riil", TeamsArm::RiilSchools),
    ("ciac", TeamsArm::CiacSchools),
    ("aia", TeamsArm::AiaProfiles),
    ("home_campus", TeamsArm::HomeCampusSections),
    ("uhsaa", TeamsArm::UhsaaDirectory),
    ("pa_piaa", TeamsArm::PiaaSchools),
    ("chsaa", TeamsArm::ChsaaSchools),
    ("tssaa", TeamsArm::TssaaSchools),
    ("bound", TeamsArm::Bound),
    ("sidearm_staff", TeamsArm::DiscoveredStaff),
];
```

Handler: `teams_arms::team_source` (`teams_arms.rs:75`) -> `arm_for` (`:68-73`) -> `dispatch`
(`:96-146`); `DiscoveredStaff` branches to `known_sites::collect` (`teams_arms/known_sites.rs:13-33`),
and `ihsa_tournament` has no generic branch (it is only reachable through its dedicated path).

`MEETS_ARMS` (`crates/census-service/src/restate_services/meets_arms.rs:19-24`):

```rust
pub(super) const MEETS_ARMS: &[(&str, MeetsArm)] = &[
    (crate::census::SOURCE, MeetsArm::MilesplitIndex),
    ("athleticnet", MeetsArm::AthleticnetHistoricalDiscovery),
    ("wiaa_results", MeetsArm::WiaaResults),
    ("wayzata", MeetsArm::Wayzata),
];
```

Meets selection predicate: `meets_arms::required` = `meet_discovery || bulk_results ||
athlete_discovery` (`meets_arms/walks.rs:82-86`), applied by `stage_runs::selected_meet_sources`
(`jurisdiction/stage_runs.rs:183`, used at `:55`).

`RESULTS_ARMS` (`crates/census-service/src/restate_services/results_arms.rs:35-40`):

```rust
pub(super) const RESULTS_ARMS: &[(&str, ResultsArm)] = &[
    (MILESPLIT, ResultsArm::MilesplitResults),
    (ATHLETICNET, ResultsArm::AthleticnetMeets),
    (TFRRS, ResultsArm::TfrrsResults),
    ("ihsa", ResultsArm::IhsaTournament),
];
```

Results selection predicate: `results_arms::required` = `arm_for(..).is_some() || bulk_results ||
pr_evidence` (`results_arms.rs:143-147`). No registry family declares `pr_evidence` (the capability
flags in use across the registry are `school_evidence`, `coach_directory`,
`public_professional_contact`, `athlete_discovery`, `athlete_profile`, `bulk_results`,
`grade_evidence`, `graduation_evidence`, `meet_discovery` - `registry/policy.rs:15-31` and the three
table files), so the results predicate in practice equals `arm || bulk_results`.

`results_arms/collection.rs:21-25` dispatches: `TfrrsResults` -> `tfrrs::collect` (`:22`),
`IhsaTournament` -> `ihsa_tournament` (`:23`, defined `:113-116`), and `MilesplitResults |
AthleticnetMeets` -> the selected-meet walk (`:24`, eligibility at `:35-46`, per-meet handler
`:66-75`, unresolved URL appended at `:88-90`). The arm handlers themselves are
`results_arms.rs:167` (`milesplit_results`) and `:191` (`athleticnet_meets`).

Contacts ladder, verbatim (21 lanes; `wire/coach_lanes.rs:53` through `:214`): AZ, CA, CT, FL, IN, IA,
LA, MA, MI, MO, NV, NJ, NY, OK, OR, SD, TX, UT, VT, VA, WA (`:54` begins AZ). Two of them arm
(`expected_status`, `wire/coach_lanes/registry_tests.rs:62-66`): SD -> `bound` (`sd.rs:13`), UT ->
`uhsaa` (`ut.rs:13`); the other 19 are `Blocked` with a recorded refusal. The 28 remaining in-scope
jurisdictions are bound by `missing::LANES` (`wire/coach_lanes/missing.rs:18-187`), each recording a
`Blocked` obligation plus a plan refusal named `<cc>_missing_coach_sources`.

**Both binds push the lane refusal into the plan unconditionally** - `missing::bind`
(`missing.rs:189-221`; push at `:213-219`) and `coach_lanes::bind` (`coach_lanes.rs:215-249`; push at
`:236-243`, with the record at `:235`), where the push is *not* gated on the lane being blocked. That
single fact drives every finding in §6.3.

`coach_contacts` (registry family, artifact CSV) has **no** stage arm: `strategies::strategy`
classifies it `Gap` (`plan/strategies.rs:26-28`) so it is refused in every plan where it applies, and
the contacts phase records the missing import explicitly (`national/contact_phase.rs:146-158`).

## 3. Matrix A - family x arm x jurisdictions

Cell notation:

- `arm:<Handler>` - reachable qualified handler (symbol named).
- `blocked:<locator>` - dispatched, terminal-by-obligation (Blocked disposition + unfinished locator).
- `delegated` - `Unknown` disposition + unfinished `acquisition-stage-outcome`.
- `refused:<kind>` - refused when the plan is recorded, before any dispatch (never swept).
- `-` - not selected by that arm.
- `cap:<flag>` - capability declared in the registry, no stage of its own.

| # | family (applicability locator) | jurisdictions making it applicable | teams | rosters | meets | results | profile | contacts |
|---|---|---|---|---|---|---|---|---|
| 1 | `aia` (data.rs:5-10) | AZ | `arm:TeamsArm::AiaProfiles` | - | - | - | - | lane AZ (`coach_lanes.rs:54-58`, Blocked) |
| 2 | `athleticlive` (data.rs:11-29) | AL, IA, IL, IN, KS, MI, MN, MO, ND, NE, OH, WI | - | - | `blocked:athleticlive:meet-csv-producer-capture` (`prerequisites.rs:31-42`) | not swept (`Gap`) | `cap:grade_evidence` | - |
| 3 | `athleticlive_athletes` (data.rs:30-48) | AL, IA, IL, IN, KS, MI, MN, MO, ND, NE, OH, WI | - | - | `blocked:no qualified bounded ... athlete-discovery manifest` (`prerequisites.rs:44-60`) | not swept (`Gap`) | `cap:grade_evidence` | - |
| 4 | `athleticnet` (data.rs:49-54) | all 49 | - | - | `arm:MeetsArm::AthleticnetHistoricalDiscovery` -> `prerequisites::athleticnet` (`prerequisites.rs:6-12`; dispatches `census_crawl::athleticnet`, fails closed when its frontier is refused) | `arm:ResultsArm::AthleticnetMeets` (`results_arms.rs:191`, `collection.rs:24,42-44`) | `cap:athlete_profile`, `cap:grade_evidence` | - (refused when no browser lane: `plan.rs:65-66`, `:147-171`) |
| 5 | `bound` (data.rs:134-139) | IA, SD | `arm:TeamsArm::Bound` (SD lane arms it, `sd.rs:13`) | - | - | - | - | lane SD (**Armed**, `registry_tests.rs:62-66`) |
| 6 | `ciac` (data.rs:55-60) | CT | `arm:TeamsArm::CiacSchools` | - | - | - | - | lane CT (`coach_lanes.rs`, LANES `:53-214`) |
| 7 | `chsaa` (data.rs:61-66) | CO | `arm:TeamsArm::ChsaaSchools` | - | - | - | - | `missing:co_missing_coach_sources` (`missing.rs:20-24`) |
| 8 | `coach_contacts` (data.rs:89-117) | AZ, CA, CO, DC, FL, GA, IA, IL, IN, KS, MI, MN, MO, ND, NE, NJ, OH, OR, TN, TX, UT, WI | `refused:EngineeringGap` (`plan/strategies.rs:26-28`) | - | - | - | - | `blocked:coach_contacts:verified-import-artifact` (`contact_phase.rs:146-158`) |
| 9 | `coach_directories` (data.rs:67-88) | NC, AL, AR, GA, MT, MS, SC, ID, NM, MD, DC, DE, TN, WY, ND | `arm:TeamsArm::CoachDirectories` | - | - | - | - | 8 of these 15 carry lane refusals: GA `:50-54`, NC `:56-60`, MD `:86-90`, SC `:92-96`, AL `:98-102`, AR `:104-108`, MS `:110-114`, MT `:122-126` (`missing.rs`) |
| 10 | `home_campus` (data.rs:118-128) | CA, FL, NJ | `arm:TeamsArm::HomeCampusSections` | - | - | - | - | - |
| 11 | `sidearm_staff` (data.rs:129-133) | CA | excluded from the teams fan-out by name (`jurisdiction/team_collection.rs:17-20`); executed by the contacts fan-out for all 49 | - | - | - | - | `arm:known_sites::collect` (`teams_arms/known_sites.rs:13-33`) -> `sidearm_staff::collect_discovered` + `school_sites::collect_contacts`; school-inventory obligation at `known_sites.rs:26-30` |
| 12 | `ihsa` (data.rs:140-145) | IL | `arm:TeamsArm::IhsaSchools` | - | - | `arm:ResultsArm::IhsaTournament` (`collection.rs:23,113-116`) | - | `missing:il_missing_coach_sources` (`missing.rs:26-30`) |
| 13 | `ks` (data.rs:146-151) | KS | `arm:TeamsArm::KsDirectory` | - | - | - | - | `missing:ks_missing_coach_sources` (`missing.rs:146-150`) |
| 14 | `milesplit` (data.rs:152-157) | all 49 | `arm:TeamsArm::MilesplitIndex` | `arm:census::collect_state_teams` + `collect_state_rosters` (`census/sweep.rs:28`, `:138`) | `arm:MeetsArm::MilesplitIndex` -> `walks.rs:31-48` | `arm:ResultsArm::MilesplitResults` (`results_arms.rs:167`, `collection.rs:24,37-41`) | `cap:athlete_discovery`, `cap:graduation_evidence` | - |
| 15 | `mpa` (data.rs:158-163) | ME | `arm:TeamsArm::MpaSchools` | - | - | - | - | `missing:me_missing_coach_sources` (`missing.rs:182-186`) |
| 16 | `mshsl` (data.rs:164-169) | MN | `arm:TeamsArm::MshslSchools` | - | - | - | - | `missing:mn_missing_coach_sources` (`missing.rs:68-72`) |
| 17 | `nces` (data.rs:226-231) | all 49 | `refused:EngineeringGap` (`strategies.rs:29`) | - | - | - | - | - |
| 18 | `ohsaa` (data.rs:170-175) | OH | `arm:TeamsArm::OhsaaSchools` | - | - | - | - | `missing:oh_missing_coach_sources` (`missing.rs:44-48`) |
| 19 | `plain_names` (data.rs:176-181) | ND, NE | `arm:TeamsArm::PlainNamesDirectories` | - | - | - | - | `missing:nd_missing_coach_sources` `:116-120`, `ne_missing_coach_sources` `:38-42` |
| 20 | `pa_piaa` (data.rs:182-187) | PA | `arm:TeamsArm::PiaaSchools` | - | - | - | - | `missing:pa_missing_coach_sources` (`missing.rs:80-84`) |
| 21 | `riil` (data.rs:188-193) | RI | `arm:TeamsArm::RiilSchools` | - | - | - | - | `missing:ri_missing_coach_sources` (`missing.rs:170-174`) |
| 22 | `state_ed` (data.rs:232-237) | NY | `refused:EngineeringGap` (`strategies.rs:30`) | - | - | - | - | - |
| 23 | `tfrrs` (data.rs:194-204) | FL, IN, NH | - | - | `blocked:no configured TFRRS page-URL frontier` (`prerequisites.rs:62-69`) | `arm:ResultsArm::TfrrsResults` -> `results_arms/collection.rs:22` | `cap:grade_evidence` | - |
| 24 | `tssaa` (data.rs:238-243) | TN | `arm:TeamsArm::TssaaSchools` | - | - | - | - | `missing:tn_missing_coach_sources` (`missing.rs:74-78`) |
| 25 | `uhsaa` (data.rs:244-249) | UT | `arm:TeamsArm::UhsaaDirectory` (UT lane arms it, `ut.rs:13`) | - | - | - | - | lane UT (**Armed**, `registry_tests.rs:62-66`) |
| 26 | `wiaa` (data.rs:214-219) | WI | `arm:TeamsArm::WiaaDirectory` | - | - | - | - | `missing:wi_missing_coach_sources` (`missing.rs:62-66`) |
| 27 | `wiaa_results` (data.rs:220-225) | WI | - | - | `arm:MeetsArm::WiaaResults` -> `walks.rs:88-105` | `delegated` (`collection.rs` has no arm; `outcomes.rs:17-30`; rebound in `pipeline/history.rs:160`) | `cap:grade_evidence` | - |
| 28 | `wayzata` (data.rs:204-214) | IA, MN, WI | - | - | `arm:MeetsArm::Wayzata` -> `walks.rs:107` | - | - | - |
| 29 | `arbiter_orgs` (data.rs:3-16 = ARBITER; table.rs:4-16) | KY, MT, NH, OK, WV | `arm:TeamsArm::ArbiterOrgs` | - | - | - | - | `missing:ky...` `:32-36`, `wv...` `:152-156`, `nh...` `:176-180`; OK/MT have no lane refusal (OK is lane OK; MT `:122-126`) |

Zero cells above are marked `arm:` without a named symbol. The five `refused:EngineeringGap`
families are refused in the recorded plan (never dispatched, never counted as executed):
`athleticlive` (`strategies.rs:20-22`), `athleticlive_athletes` (`:23-25`), `coach_contacts`
(`:26-28`), `nces` (`:29`), `state_ed` (`:30`). `athleticlive` and `athleticlive_athletes` are still
*selected into the meets stage* because the meets stage chains registry-known refused slugs and
`meets_arms::required` is true for both (`meets_discovery`/`athlete_discovery`,
`walks.rs:82-86`), so their blocked censuses are produced even though the plan refuses them.

## 4. Matrix B - per-jurisdiction plan composition

Applicable families per jurisdiction = 3 universal (`milesplit`, `athleticnet`, `nces`) + the scoped
extras below (from the rows quoted in §3). `R` marks a Gap-refused family (plan strategy `Gap`, never
swept). Every jurisdiction additionally carries one `<cc>_missing_coach_sources` refusal row from the
coach-lane bind (`coach_lanes.rs:236-243` or `missing.rs:213-219`) - see §6.3 for why that matters.

| Jurisdiction | families | count | Gap-refused among them |
|---|---|---|---|
| AL | + athleticlive R, athleticlive_athletes R, coach_directories | 6 | 3 |
| AR | + coach_directories | 4 | 1 |
| AZ | + aia, coach_contacts R | 5 | 2 |
| CA | + coach_contacts R, home_campus, sidearm_staff | 6 | 2 |
| CO | + chsaa, coach_contacts R | 5 | 2 |
| CT | + ciac | 4 | 1 |
| DC | + coach_contacts R, coach_directories | 5 | 2 |
| DE | + coach_directories | 4 | 1 |
| FL | + coach_contacts R, home_campus, tfrrs | 6 | 2 |
| GA | + coach_contacts R, coach_directories | 5 | 2 |
| IA | + athleticlive R, athleticlive_athletes R, bound, coach_contacts R, wayzata | 8 | 4 |
| ID | + coach_directories | 4 | 1 |
| IL | + athleticlive R, athleticlive_athletes R, coach_contacts R, ihsa | 7 | 4 |
| IN | + athleticlive R, athleticlive_athletes R, coach_contacts R, tfrrs | 7 | 4 |
| KS | + athleticlive R, athleticlive_athletes R, coach_contacts R, ks | 7 | 4 |
| KY | + arbiter_orgs | 4 | 1 |
| LA | (none) | 3 | 1 |
| MA | (none) | 3 | 1 |
| MD | + coach_directories | 4 | 1 |
| ME | + mpa | 4 | 1 |
| MI | + athleticlive R, athleticlive_athletes R, coach_contacts R | 6 | 4 |
| MN | + athleticlive R, athleticlive_athletes R, coach_contacts R, mshsl, wayzata | 8 | 4 |
| MO | + athleticlive R, athleticlive_athletes R, coach_contacts R | 6 | 4 |
| MS | + coach_directories | 4 | 1 |
| MT | + arbiter_orgs, coach_directories | 5 | 1 |
| NC | + coach_directories | 4 | 1 |
| ND | + athleticlive R, athleticlive_athletes R, coach_contacts R, coach_directories, plain_names | 8 | 4 |
| NE | + athleticlive R, athleticlive_athletes R, coach_contacts R, plain_names | 7 | 4 |
| NH | + arbiter_orgs, tfrrs | 5 | 1 |
| NJ | + coach_contacts R, home_campus | 5 | 2 |
| NM | + coach_directories | 4 | 1 |
| NV | (none) | 3 | 1 |
| NY | + state_ed R | 4 | 2 |
| OH | + athleticlive R, athleticlive_athletes R, coach_contacts R, ohsaa | 7 | 4 |
| OK | + arbiter_orgs | 4 | 1 |
| OR | + coach_contacts R | 4 | 1 |
| PA | + pa_piaa | 4 | 1 |
| RI | + riil | 4 | 1 |
| SC | + coach_directories | 4 | 1 |
| SD | + bound | 4 | 1 |
| TN | + coach_contacts R, coach_directories, tssaa | 6 | 2 |
| TX | + coach_contacts R | 4 | 1 |
| UT | + coach_contacts R, uhsaa | 5 | 2 |
| VA | (none) | 3 | 1 |
| VT | (none) | 3 | 1 |
| WA | (none) | 3 | 1 |
| WI | + athleticlive R, athleticlive_athletes R, coach_contacts R, wayzata, wiaa, wiaa_results | 9 | 4 |
| WV | + arbiter_orgs | 4 | 1 |
| WY | + coach_directories | 4 | 1 |

Column check: 49 rows; family-jurisdiction pairs = 3x49 + 95 = 242, matching §1.

Meets-arm coverage by jurisdiction (families the meets stage actually selects, i.e.
`meets_arms::required` over `plan.sweepable` + registry-known refused):

| Stage | families | jurisdictions |
|---|---|---|
| meets, armed | milesplit (all 49), athleticnet (49 where swept), wiaa_results (WI), wayzata (IA, MN, WI) | see §3 |
| meets, blocked obligation | athleticlive (12), athleticlive_athletes (12), tfrrs (FL, IN, NH) | `prerequisites.rs:14-72` |
| results, armed | milesplit (all 49), athleticnet (49 where swept), tfrrs (FL, IN, NH), ihsa (IL) | §2 |
| results, delegated (no arm) | wiaa_results (WI; rebound from the acquired meet census) | `pipeline/history.rs:160` |

## 5. Silent-default audit

Every "unsupported stage treated as complete" candidate in the durable path, with the verdict from
the source:

| # | Candidate | Locator | Verdict |
|---|---|---|---|
| 1 | meets stage: `if !required(source)` early return | `meets_arms/walks.rs:54-80` vs `required` `:82-86` | Not reachable in the durable path: `selected_meet_sources` filters with the same `required` (`stage_runs.rs:183-189`). Defensive only; and `MeetCensus` default is **not** terminal (`census/meets.rs:92-99` requires non-empty sources all complete). |
| 2 | meets stage: unknown selected slug | `meets_arms/walks.rs:54-80` (`unarmed` catch-all) | Fails closed: `prerequisites::unarmed` (`prerequisites.rs:14-29`) returns Blocked + `<slug>:historical-discovery-arm`. |
| 3 | meets unarmed preconditions for athleticlive/athletes/tfrrs | `prerequisites.rs:31-42`, `:44-60`, `:62-69` | Blocked disposition + named prerequisite (`blocked` helper `:71-...`). |
| 4 | teams stage: sources with no teams arm are dropped from the fan-out | `jurisdiction/team_collection.rs:17-20` | Not currently live (every sweepable teams-strategy family has an arm; non-armed families are Gap-refused). If it ever were live the stage would not report Complete: the disposition is computed from the selected set at `team_collection.rs:21`. |
| 5 | teams stage: `arm_for(source).is_none()` in `team_source` | `teams_arms.rs:75-93` | Fails closed via `jobs::require_stage_arm` for a Wired slug, and via the `Dispatch::Unwired` terminal error otherwise. |
| 6 | results: a required slug with no results arm | `results_arms.rs:161-162` (`delegated_source` after `assert_some_stage_arms`) | Not silent: `Unknown` + unfinished `acquisition-stage-outcome`; `source_complete` false (`outcomes.rs:5-15`). |
| 7 | results: capability-required families the plan refuses | `results_arms.rs:143-147` vs `plan/strategies.rs:19-22` | Denominator illusion: `required()` names 6 families, the plan can sweep 5 (see §6.2.2). No false completion. |
| 8 | contacts: `TeamsSource.contacts` accepts any phase-matching request and dispatches by `request.source` | `jurisdiction/team_source.rs:230-243` | The contacts fan-out always sends `sidearm_staff` (`contact_phase.rs:20-23`), so `TeamsArm::DiscoveredStaff` is the only reachable branch; `known_sites::collect` owes `<cc>/school-sites/inventory` when the jurisdiction has no stored school inventory (`known_sites.rs:26-30`). Fails closed. |
| 9 | contacts: `coach_contacts` absent from any stage | `plan/strategies.rs:26-28`, `contact_phase.rs:146-158` | Explicit Blocked obligation with unfinished `coach_contacts:verified-import-artifact`; also refused in the plan. |
| 10 | open work: a sweepable family with neither a teams object, a meets entry nor a results entry | `open_work/inventory.rs:189-206` (`source/<slug>/unmeasured`) | Explicit `Unknown` obligation. |
| 11 | open work: `Kind::requires` returns `true` for an unregistered slug | `open_work/history.rs:19-25` (`is_none_or`) | Fails closed (a slug with no descriptor is treated as required), not open. |
| 12 | coverage writer accepts any subset of the stages | `census-report/src/report/coverage/population.rs` (`STAGES`); `national/coverage.rs:35` emit set | Validates for *unknown* keys only; a row may declare just `registered`. Partial denominators are accepted, not silently completed, but no gate requires all stages. |
| 13 | seal: `WorkbookCheck` booleans | `census/state/seal/workbook.rs:37-42` | Set `true` unconditionally on the strength of the preceding `open_publication` (which does run `verify_frozen`). The four items are one verification projected four ways, not four independent measurements. |
| 14 | seal: empty-population short-circuit | `census/state/evidence.rs:99-109` | `EvidenceDurable`/`ConflictsRetained`/`CalculationsReproducible`/`RetriesRepresented` are only demanded when the corresponding denominator is non-zero (`counts.athletes > 0`, `counts.cohort_performances > 0`, `counts.schools > 0`). An all-empty run owes no evidence items; the obligation surface for it is `silent_sources`, which no `AcceptanceItem` inspects (§6.1). |

No live silent completion was found inside the four collection arms: every branch that cannot do the
work either refuses at plan time, or emits Blocked/Unknown/Partial with an unfinished locator.

## 6. F07 / F11 / F12 reconciliation

Plan lines: F07 = `docs/NATIONAL-CENSUS-PLAN.md:356`, F11 = `:360`, F12 = `:361` (F-table `:349-364`).

### 6.1 F07 - "Source/program/season obligations and history remain explicit even with no accepted athletes"

Reconciling handlers:

- Meets: every selected source yields a `MeetSourceRows` row; rows built for a complete report carry
  `withheld: Some(report.rejections)` and `unresolved` defaulting to zero on completion
  (`meets_arms.rs:201-214`), failures are explicit (`:226-240`), and terminal requires a non-empty,
  all-complete source list (`census/meets.rs:92-99`).
- Results: `ResultsSourceRows` per required source with `rows: Option<usize>`
  (`results_arms.rs:62-64`, aggregation `collection.rs:134-138`), and `is_terminal` requires each
  required slug exactly once and complete (`results_arms.rs:67-80`, `:113-140`).
- Contacts: `ContactResearchOutcome::CompletedEmpty | CompletedClaims` map to `Complete` while
  `Unattempted | Stale` map to `Unknown` (`open_work/store_evidence.rs:267-278`); a jurisdiction with
  no school inventory gets `<cc>/contact-research/school-inventory-unmeasured`
  (`store_evidence.rs:125-139`).
- Open work names zero-observation-but-complete endpoints rather than dropping them:
  `open_work.rs:125` (`silent`) -> field at `open_work.rs:108`, carried in
  `wire/open.rs:56-62` -> `RetainedFindings.silent_sources`
  (`census/state/evidence/recorded.rs:25-34`) -> seal digest input
  (`census/state/seal_digest.rs:17-21`, rendered `:31-32`), assembled at
  `census/seal/assembly.rs:76-79`.
- Team objects: `StageOutcome.records` + `disposition`, projected per family in
  `open_work/inventory.rs:254-293`.

Gap (explicit, source-backed): `SealEvidence::open_items` (`census/state/evidence.rs:85-126`) tests
only `open.jurisdiction_sweeps`, `open.source_objects`, `open.cohort_decisions`,
`open.identity_candidates`, the four `retained.*` counts against non-zero denominators
(`:99-109`), `workbook.mapped_athletes` (`:111-113`), the four workbook booleans (`:114-125`) and
`run` (`:126`). `silent_sources`, `gaps` and `source_failures` are **digest inputs only** and never
fail the seal; and with `counts.athletes == 0` the evidence/conflict/calculation/retry items are
skipped. Verdict: **reconciling as recorded, mismatching as gated** - F07 obligations are explicit in
the retained findings, but no acceptance item fails closed on a silent source.

### 6.2 F11 - "Denominators include unresolved/unfinished coverage; source failure is not absence"

Mismatches found, each with its evidence lines:

1. **Wisconsin results can never be terminal** `[INFERENCE chain]`.
   Premises: (a) WI's plan sweeps `wiaa_results` (applicability `data.rs:220-225`, strategy not Gap:
   `plan/strategies.rs:9-18`); (b) `results_arms::required` is true for it (`bulk_results`,
   `from_mshsl.rs:67-72`; predicate `results_arms.rs:143-147`), so it is a required source
   (`results_arms.rs:113-140`) and, having no results arm, is collected as `delegated_source`
   (`results_arms.rs:161-162`); (c) `bind_acquired_results` copies `rows`, `disposition`, `errors`,
   `notes`, `unfinished`, `withheld`, `unresolved` from the acquired meet census but **not**
   `resolution` (`pipeline/history.rs:160-200`); (d) `source_complete` requires
   `resolution.is_some_and(|value| value.unresolved == 0)` (`results_arms/outcomes.rs:5-15`); (e)
   `is_terminal` requires `per_source.iter().all(source_complete)` (`results_arms.rs:67-80`).
   Consequence: `state.history.results[year].is_terminal()` is false for WI forever, and
   `open_work::history::result` reports it `Partial` forever (`open_work/history.rs:108-136`;
   `uncertain` includes `resolution.is_none()`). The happy-path fixture that asserts the opposite
   hand-builds its rows (`open_work/tests/collection.rs:4-52`) and its plan is not producible by the
   real binds (see 6.3).
2. **Meets denominator 7 vs open-work meets denominator 5.** The meets stage selects
   `{milesplit, athleticnet, wiaa_results, wayzata}` + `{athleticlive, athleticlive_athletes, tfrrs}`
   (`walks.rs:82-86` applied at `stage_runs.rs:183-189`). The open-work projection credits a meets
   source-object row only when `meet_discovery || meets_arms::arm_for(..)` holds
   (`open_work/history.rs:19-25`). `athleticlive_athletes` (athlete_discovery only) and `tfrrs`
   (bulk_results/athlete_discovery, `from_mshsl.rs:47-53`) are therefore absent from the meets
   numerator, although the stage produced blocked censuses for them. `tfrrs` still gets results rows;
   `athleticlive_athletes` falls through to the generic `Unknown` obligation
   (`open_work/inventory.rs:202-206`).
3. **Results denominator 6 vs swept 5.** `required()` counts capability-or-arm matches
   (`results_arms.rs:143-147`): milesplit, athleticnet, tfrrs, ihsa, wiaa_results, athleticlive. The
   last is Gap-refused and never sweepable (`plan/strategies.rs:19-22`), so in any real plan
   `required_sources` has at most 5 members and `athleticlive`'s `delegated_source` branch is
   unreachable. No false completion - but the family's results obligation is discharged by the
   refusal, not by the stage.
4. **`delegated_source` is the only "unarmed but required" path**, and it is honest (Unknown +
   unfinished) - the defect is 6.2.1's missing `resolution`, not the delegation itself.

Reconciling parts (verified against source): `source_complete` includes `withheld == Some(0)`,
`unfinished` empty, `unresolved == Some(0)`, `resolution.unresolved == 0`
(`results_arms/outcomes.rs:5-15`); results rows append the exact meet locator when a row is not
complete (`results_arms/collection.rs:88-90`); meets rows carry `withheld`/`unresolved`
(`meets_arms.rs:201-214`); `open_work::history::meet/result` downgrade complete-with-uncertainty to
`Partial` (`open_work/history.rs:100-104`, `:132-136`). F11's "source failure is not absence" holds
at the row level; its denominator arithmetic does not close for WI and is short by two families on
the meets side.

### 6.3 Two gates that count the same refusals with opposite polarity

`[INFERENCE chain]`, premises named:

- Every recorded plan carries at least one refusal that is **not** a registry family: the coach-lane
  refusal `<cc>_missing_coach_sources`, pushed by `coach_lanes::bind` for all 49 jurisdictions
  (`coach_lanes.rs:236-243` for the 21 armed/blocked lanes, `missing.rs:213-219` for the other 28).
  The push is unconditional: the test `jpx41_an_accepted_coach_refusal_settles_the_plan_but_a_recorded_one_does_not`
  binds Oregon's lane and shows the refusal is present *before* acceptance
  (`open_work/tests/collection.rs:145-188`).
- `plan_covers` (`open_work/inventory.rs:42-64`) requires the plan's sweepable + refused slugs to be
  exactly the applicable family set. With the extra coach-lane refusal the count is
  `applicable + 1`, so `plan_covers` is false for every jurisdiction whose state carries a bound
  lane. Its own harness builds plans without lane refusals
  (`open_work/tests.rs`, `open_work/tests/collection.rs:4-52`).
- Consequence A: `open_work::reply`'s `enumerated` is false (`open_work.rs:79-82`), so
  `source_objects: Option<u64>` stays `None` (`open_work.rs:98-104`) and
  `AcceptanceItem::SourceObjectsTerminal` is permanently open (`census/state/evidence.rs:90-92`).
- Consequence B: `collection_is_complete` fails at `plan_covers` (`open_work.rs:213-230`) for the
  same reason, so `jurisdiction/report.rs:58-62` refuses every jurisdiction report
  ("jurisdiction source work remains incomplete after the stages ran").
- Consequence C (independent of A/B): `refusals_settled` requires *every* `plan.refused` entry to be
  an accepted coach refusal (`open_work.rs:232-240`), and `accepted_coach_refusal` first asks
  `wire::coach_lanes::refusal_slug(jurisdiction)` (`open_work.rs:242-250`), which searches only the
  21-row `LANES` table (`coach_lanes.rs:44-51`). For the 28 `missing::LANES` jurisdictions it returns
  `None`, so no acceptance can ever settle their refusals - the test asserts exactly this negative
  for Wisconsin (`open_work/tests/collection.rs:186`). Gap refusals (`nces`, `state_ed`,
  `coach_contacts`, and `athleticlive`/`athleticlive_athletes` in 12 states) can never settle either,
  because their slugs never equal the jurisdiction's lane slug.
- Consequence D (independent of A-C): `JurisdictionStages::terminal()` requires
  `refused_sources == 0` (`census/state/open.rs:18-32`, the clause at `:31`), and `refused_sources`
  is `plan.refused.len()` (`open_work.rs:202-206`). Any refusal therefore keeps the jurisdiction
  non-terminal forever - `owing()` even names `refused_sources` as its own stage
  (`census/state/open.rs:51`; test case at `census/state/tests.rs:716-721`) - so `open_work`'s
  `jurisdiction_sweeps` cannot reach 0 and `AcceptanceItem::JurisdictionSweepsTerminal`
  (`evidence.rs:87-89`) stays open, which is the seal's first gate
  (`restate_services/census/requests.rs:28-56` builds the open-work request from the seal request,
  including the journal's `silent_sources` at `:53`; the items are then evaluated at
  `census/state/evidence.rs:87-92`).

Verdict: F11's denominator arithmetic is not just incomplete, it is **unsatisfiable in the two places
where it is enforced** (report publication and seal completion) as long as any source is refused.
This is the highest-severity item in this inventory.

### 6.4 F12 - "Fresh source-discovered population reconciles to generated outputs, not seed-workbook accounting"

Reconciling chain that exists:

1. The national pass publishes per-jurisdiction coverage rows from the run's own target list:
   `national/coverage.rs:13` (`publish`) -> `:35` (`coverage_rows`) -> the coverage report writer
   (`census-report/src/report/coverage/population.rs`, `STAGES` list and `publish_coverage_rows`),
   which validates stage keys and writes `Table::Coverage` keyed on `store.derived_generation()`.
2. Export reads the published rows back as coverage evidence
   (`census-report/src/export/dataset.rs`).
3. `CoverageReport::reconcile` checks published-vs-read totals, lifecycle ordering, contact and state
   reconciliation, and per-stage source population sums (`census-report/src/report/coverage.rs`).
4. The workbook's Coverage sheet is re-derived from the dataset and compared cell-by-cell
   (`census-report/src/workbook/verify/meta/coverage.rs`), and `open_publication` runs that
   verification before the seal reads `WorkbookCheck`
   (`census-report/src/workbook/publication/manifest.rs`).

Mismatches / unproven parts:

- **Only three of the stages are published by the run.** `coverage_rows` emits `registered=1`,
  `applicable=1` and conditionally `attempted=1` (`national/coverage.rs:35`), and the writer's
  `StageTotals` only reports a stage when every row declares it
  (`census-report/src/report/coverage/population.rs`). For a fresh run
  `SourcePopulation.captured/parsed/accepted/withheld` are therefore `None` - the F11
  "unresolved/unfinished coverage" denominators are not part of the F12 population reconciliation
  at all.
- **The seal's four F12 booleans are asserted, not derived**:
  `census/state/seal/workbook.rs:37-42` sets
  `counts_reconciled/coverage_reconciled/metrics_reconciled/export_verified = true` (and
  `discrepancies = Vec::new()`) immediately after `open_publication` returns. The verification that
  justifies them is real (`verify_frozen`), but the seal cannot distinguish "verified" from "not
  measured" for any individual item.
- **Workbook-to-store lineage is checked, not recomputed**: `open_publication` binds manifest to
  store lineage/scope/cohort and decodes the frozen input
  (`census-report/src/workbook/publication/manifest.rs`); nothing re-runs the coverage report
  against the live store at seal time, so "reconciles to generated outputs" is dataset-internal for
  the source-population columns.

Verdict: F12 is **partially reconciling** - workbook, dataset, sidecars and manifest do reconcile
against each other from a fresh (non-seed) input generation, and `mapped_athletes`/class-cohort
checks exist (`census/state/evidence.rs:111-113`); but the source-discovery denominators that the
plan's §2 asks for are only three self-declared counters per jurisdiction.

## 7. Arms-module coverage gaps

**`results_arms` lacks a handler for which family:** `wiaa_results` (WI) and `athleticlive` (12
states) are results-required but have no arm; both are reduced to
`results_arms::delegated_source` (`results_arms.rs:161-162`, `outcomes.rs:17-30`). `athleticlive` is
unreachable (plan-refused); `wiaa_results` is reachable in WI and, after the meets rebind, still
cannot satisfy `source_complete` (§6.2.1). Registered families with a results obligation and no
results arm, by capability: `wiaa_results` (`bulk_results`, `from_mshsl.rs:67-72`) and
`athleticlive` (`bulk_results`, `through_milesplit.rs:19-24`).

**`teams_arms` lacks a handler for which family:** none of the families the teams stage can select.
The teams fan-out filters on `arm_for(..).is_some()` (`jurisdiction/team_collection.rs:17-20`), and
the registry families with **no** teams arm are `athleticnet`, `athleticlive`,
`athleticlive_athletes`, `coach_contacts`, `nces`, `state_ed`, `tfrrs`, `wayzata`, `wiaa_results`
(9 of 29). Every one of them is either Gap-refused in the plan (`athleticlive`,
`athleticlive_athletes`, `coach_contacts`, `nces`, `state_ed`), or is collected by another stage
(`athleticnet`, `wayzata`, `wiaa_results`, `tfrrs`) - so the filter is not silently dropping work
today, but it is the filter, not an obligation, that protects it. `sidearm_staff` is additionally
excluded from the teams fan-out by name (`team_collection.rs:18`, `census_crawl::sidearm_staff::SOURCE_ID`)
and is executed by the contacts fan-out instead.

**`meets_arms` lacks a handler for which family:** `athleticlive`, `athleticlive_athletes`, `tfrrs`
- all three reach `meets_arms::prerequisites::unarmed` and return Blocked censuses with a named
prerequisite (`prerequisites.rs:14-69`). The meets stage has no unarmed-but-complete path (§5.1-3).

## 8. Outstanding obligations (for beads / next pass)

1. **O1 (F11, high).** `bind_acquired_results` must populate `resolution` (or `source_complete` must
   stop requiring it for acquisition-delegated sources), otherwise the WI results stage can never be
   terminal. `pipeline/history.rs:160-200`, `results_arms/outcomes.rs:5-15`.
2. **O2 (F11, high).** Decide the meets denominator: either add
   `athlete_discovery`/`bulk_results` to `open_work::history::Kind::requires(Meets)`, or drop those
   families from `meets_arms::required`. `open_work/history.rs:19-25` vs `meets_arms/walks.rs:82-86`.
3. **O3 (F11, high).** Model non-family refusals in the plan denominator: `plan_covers`
   (`open_work/inventory.rs:42-64`) currently compares family counts against a plan that also carries
   coach-lane refusals (pushed unconditionally at `coach_lanes.rs:236-243` /
   `missing.rs:213-219`), so it is false for every real jurisdiction.
4. **O4 (F11, high).** Give `accepted_coach_refusal` a `missing::LANES` lookup (or make
   `refusal_slug` cover all 49; `coach_lanes.rs:44-51` searches only `LANES`), otherwise the 28
   missing-lane jurisdictions can never settle their refusals.
5. **O5 (F07/F11, high).** Reconcile `JurisdictionStages::terminal()`'s `refused_sources == 0`
   (`census/state/open.rs:18-32`) with `collection_is_complete`'s "accepted refusal is settled"
   (`open_work.rs:232-249`). Today the sweep-level gate can never reach zero while any refusal
   exists, so `AcceptanceItem::JurisdictionSweepsTerminal` and `SourceObjectsTerminal` are
   permanently open.
6. **O6 (F12, medium).** Publish all `STAGES` from the run (or fail closed for unmeasured stages) so
   captured/parsed/accepted/withheld participate in coverage reconciliation.
   `national/coverage.rs:35`, `census-report/src/report/coverage/population.rs`.
7. **O7 (F07, medium).** Add an acceptance item (or gate) for `RetainedFindings.silent_sources`,
   `gaps` and `source_failures`; they are currently digest-only.
   `census/state/evidence.rs:85-126`, `census/state/seal_digest.rs:17-32`.
8. **O8 (F12, medium).** Split `WorkbookCheck`'s four booleans from the single `open_publication`
   success, or record the individual verification results, so the seal reports what was measured.
   `census/state/seal/workbook.rs:17-43`.
9. **O9 (policy, high).** The registry still plans and the results arm still fetches `tfrrs`
   (`applicability/table/data.rs:194-204`; `results_arms.rs:38`), while the current wave's staged
   qualification classifies **every** DAT/TFRRS path `REFUSED (terms)` - unconditional
   "no automated means ... for any purpose without our express written permission" clause and no
   permission on file (`research/sources/national-aggregators/TFRRS-DAT-PATH-QUALIFICATION-2026-10-12.md`,
   sections "DAT/TFRRS: the Terms of Use refuse automated acquisition" (line 16) and
   "Classification: REFUSED (terms)" (lines 40-41)). Nothing in `crates/` encodes a terms refusal
   (the only hits for "terms of use" are school-site junk regexes:
   `census-crawl/src/school_sites/parse/patterns.rs:155`,
   `census-service/src/school_sites/contacts/vocabulary.rs:267`). Either the family must be refused
   in `plan/strategies.rs` or the refusal must be recorded with its evidence.
10. **O10 (denominator hygiene, low).** `results_arms::required` advertises `pr_evidence`, which no
    registry family declares (`results_arms.rs:143-147`); the clause is dead code that inflates the
    apparent results denominator.
11. **O11 (test fidelity, medium).** `open_work/tests/collection.rs::completed_collection` (`:4-52`)
    and `open_work/tests.rs` build plans without coach-lane refusals and (for WI) with a
    producer-unreachable terminal results outcome, so the "completed jurisdiction" assertion does not
    exercise the production plan shape. Reproduce the fixture with the real binds' output instead.

## 9. Unverified / residual uncertainty

- No command was executed, so the arithmetic gates in §6.3 are `[INFERENCE chain]`: each premise is
  a source line, but the end-to-end effect (plan recorded by the real binds -> `plan_covers` false ->
  report refused) should be confirmed by running `open_work/tests.rs` variants with a lane bound,
  and by the `wire/coach_lanes/registry_tests.rs` group tests (`:163` `check_group` binds a lane and
  calls `assert_decision`, but never calls `plan_covers`).
- `athleticnet` is refused only while no browser lane is configured; I did not verify what a
  configured lane changes about `open_publication`/coverage.
- I did not read the collection adapters one by one (e.g. `census_crawl::milesplit`, `athleticnet`,
  `ihsa::tournament`, `wiaa_results`, `wayzata`, `tfrrs` collectors); "reachable qualified handler"
  is claimed at the dispatch level (`arm_for` -> named function), which is the level the acceptance
  asks for. Adapter-internal qualification is owned by the per-source lanes' `SOURCE_REPORT.md`
  files.
- The `census-report` chain (§6.4) was audited from the national writer's call site and the
  previously read `population.rs`/`manifest.rs`; I did not re-derive every line number in that crate
  in this pass (concurrent writers are active there).
- The 28 `missing::LANES` reasons are quoted elsewhere (work orders); I verified their slugs, kinds
  and the `Blocked` obligation shape, not each reason's research provenance.
- `docs/NATIONAL-CENSUS-PLAN.md` line numbers are from the 540-line revision read today; F07/F11/F12
  are at `:356`, `:360`, `:361`.
