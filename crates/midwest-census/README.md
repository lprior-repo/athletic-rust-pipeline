# midwest-census

An independent census of Midwest high-school track & field / cross-country athletes, schools, teams,
coaches, meets, events and performances.

**Athletic.net and its AthleticLIVE mirror are outside the core contract.** Every number in the
`core` rows below was produced with both switched off: the census reads state associations,
MileSplit-style rosters, official result artifacts and timing-provider schedules that the pipeline
fetches itself. The `all_sources` scope adds the two AthleticLIVE modules (a results mirror and its
athlete index) purely as comparison, and the difference between the two rows is the measured
marginal coverage of the source this pipeline refuses to depend on.

## Run

```text
cargo run --release -p midwest-census -- <command>

  fetch           Fetch a single URL through the polite fetcher (robots-enforced, cached)
  sites           List the registered MileSplit state sites
  teams           Fetch (and cache) team indexes for the given states
  collect         Walk rosters and emit canonical entities for the given states
  import-coaches  Import the researched official coach-contact CSV into canonical entities
  provider        Run one association contact adapter by name
  consolidate     Merge append logs into `out/*.jsonl` snapshots
  report          Compute the measured census from consolidated snapshots

Global: --store <dir> (default var/midwest-census), --delay-ms <n>, --user-agent <ua>
```

A full cycle is `collect` → `provider <name>` per adapter → `consolidate` → `report`
(`report --core` for the Athletic.net-free scope). Every adapter is resumable: a unit of work is
journaled with its parser version, and a re-run skips what an unchanged parser already produced.
Each provider takes `--seasons`, `--limit` and `--refresh`.

## Measured, 2026-09-20

| scope | athletes | Class of 2027 | boys | girls | grade-evidenced | profile URL | coach | coach email |
|---|---|---|---|---|---|---|---|---|
| **core** (Athletic.net off) | 651,736 | **146,858** | 80,896 | 65,751 | 100% | 142,916 (97.3%) | 37,179 (25.3%) | 28,929 (19.7%) |
| all sources | 787,584 | 190,087 | 105,755 | 84,034 | 100% | 176,885 (93.1%) | 43,201 (22.7%) | 33,340 (17.5%) |

Core independently reaches **77.3%** of the all-source Class-of-2027 population. 27,580 canonical
coaches are held in total.

| state | schools | athletes | Class of 2027 | boys | girls | with coach | with coach email |
|---|---|---|---|---|---|---|---|
| OH | 1,504 | 87,765 | 26,998 | 14,414 | 12,507 | 313 | 313 |
| IL | 1,889 | 69,493 | 19,392 | 10,856 | 8,521 | 6,432 | 6,244 |
| WI | 1,027 | 118,031 | 19,281 | 10,323 | 8,929 | 14,699 | 13,646 |
| MI | 1,493 | 94,771 | 16,176 | 9,285 | 6,891 | 0 | 0 |
| MO | 1,037 | 42,226 | 13,218 | 7,377 | 5,839 | 0 | 0 |
| IN | 1,108 | 40,846 | 11,909 | 6,430 | 5,402 | 0 | 0 |
| MN | 1,016 | 54,084 | 11,261 | 6,147 | 5,114 | 9,338 | 8,629 |
| KS | 836 | 29,950 | 9,252 | 5,258 | 3,993 | 0 | 0 |
| IA | 913 | 49,024 | 8,738 | 4,924 | 3,804 | 76 | 19 |
| NE | 678 | 35,800 | 5,921 | 3,248 | 2,673 | 4,971 | 0 |
| SD | 427 | 17,134 | 2,669 | 1,518 | 1,151 | 239 | 78 |
| ND | 339 | 12,612 | 2,043 | 1,116 | 927 | 1,111 | 0 |

Meets: **1,532 core** (WI 796, MN 209, IA 85, otherwise unresolved) against 11,007 across all
sources, of which 9,593 carry an Athletic.net meet id - that id is retained as an enrichment key and
is never dereferenced by a core run.

## Source tiers

| tier | source | adapter | what it yields |
|---|---|---|---|
| A | WIAA school/team/coach database (WI) | `wiaa` | schools, teams, coach names and email |
| A | WIAA result archive (WI) | `wiaa_results` | grade-bearing result rows, schools, class-of-2027 evidence |
| A | MSHSL (MN) | `mshsl` | schools, activities director, published staff email |
| A | IHSA (IL) | `ihsa` | member schools, per-school head-coach roles and email |
| A | OHSAA (OH) | `ohsaa` | schools, head coach names by sport |
| A | KSHSAA (KS) | `ks` | member schools, athletic-director name and email |
| A | NDHSAA + NSAA (ND, NE) | `plain_names` | school universes, coach names (no email published) |
| A | researched official contact graph | `coach_contacts` | artifact import: school / sport / role + published email |
| B | MileSplit-style state sites | `milesplit` (driven by `teams` and `collect`) | rosters, graded athletes, profile URLs |
| D | vendor result artifacts | `result_file` dispatching `hytek`, `compiled`, `xc`, `raceday` | performances with grade evidence |
| E | Wayzata Results (MN / IA / WI timer) | `wayzata` | meet inventory from published schedules |
| - | AthleticLIVE mirror | `athleticlive`, `athleticlive_athletes` | **non-core**: comparison only |

### Result-artifact parsing

`result_file` dispatches on the detected vendor layout and shares one header-anchor rule across
`hytek`, `compiled` and `xc`. On the Wisconsin archive that rule takes 96 parsed artifacts to 1,740
(compiled 763, cross-country 380, Hy-Tek 597) and produces 834,254 result rows, 264,167 of which
carry a grade - enough to add 4,323 class-of-2027 athletes to core without a single Athletic.net
request.

### Wayzata schedules

The provider publishes one server-rendered schedule table per sport and season. Its `/links/<slug>`
pages render AthleticLIVE, so the adapter keeps the slug as a `TimerMeet` key and the URL as a
source URL while never fetching either: the schedule row is the evidence. Meet identity is the
platform's own (`state + date + normalized name`), so a meet listed here reconciles with the same
meet arriving from an association artifact. Venues are resolved to a state by a recurring-site table
or, failing that, by the consolidated school snapshot restricted to the provider's region - a
nationwide search turns "Austin HS" into a three-way tie, while the provider's Austin is the
Minnesota one. Unresolved venues are filed under `??` rather than guessed: on the 2026 schedules,
304 of 537 rows resolve (95 sites, 209 schools).

## Isolation contract

- `report::NON_CORE_SOURCE_IDS` names the two AthleticLIVE modules; `core` scope drops any evidence
  they produced. Test: `report::tests::core_scope_keeps_only_non_athletic_net_evidence`.
- Core adapters mention Athletic.net only in prose, never as a request target or a parser input.
- Cross-source agreement (`multisource`) is an all-source metric; it is 0 in core scope by
  construction, because core scope holds one independent view of each athlete.

## Known limits

- Coach coverage is uneven by design: the states whose association publishes a coach directory (WI,
  MN, IL, OH, NE, ND) have it, IA and SD have partial coverage from the researched contact graph, and
  MI, MO, IN and KS have none yet.
- 442 core meets have no resolved state, and core meet inventory outside Wisconsin, Minnesota and
  Iowa depends on which association artifacts have been ingested.
- Grade evidence is "grade observed in a source", not a verified graduation year; the platform keeps
  `GradYear` and `ObservedGrade` as separate fields for exactly that reason.
- 183 tests pass. The adapters added in this workspace (`wayzata`, `compiled`, `xc`) are clippy-clean;
  the crate still carries 17 pre-existing warnings in `ohsaa`, `plain_names`, `wiaa`, `net`, `mshsl`,
  `hytek`, `athleticlive_athletes`, `report` and `census`.
