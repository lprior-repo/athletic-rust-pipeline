# Acceptance audit — measured at seal `7b41a264` (2026-09-22)

**Rebuild note (2026-09-22, after this audit was written).** The workbook below was rebuilt at 11:53 by
`tools/run_pipeline.sh` after the coach-plane import and the verified fragment union
(`data/coach-contacts.csv` 3,444 → 6,215 rows; store coaches 29,294 → 31,488): the on-disk
`var/midwest-census/out/midwest-census-2026-09-22.xlsx` (and its copy in `reports/`) now digests
`c63f7fb4f38f55167fc3019e722bff122bbac2fc156410552935d786c169da21`, so the `7b41a264` seal this audit
measures is **no longer on disk**. The numbers below remain the audit of that earlier seal — the coach rows
they report (43,201 Co2027 with a coach / 33,340 with a coach email) are the pre-import generation; the
current generation is in `synthesis/01-acceptance-answers.md` §Q5/§Q6 and `reports/census-by-state*.csv`
(57,115 / 46,185 on the 12-state filter).

The brief's nine acceptance questions, answered from the sealed workbook
`var/midwest-census/out/midwest-census-2026-09-22.xlsx` (digest
`7b41a26465e24c2e3c21de1b22f3e66a4e5a139b815793c5a162497002fc2723`, every §70 item satisfied, all
reconciliation rows `reconciled`).

Scope vocabulary, straight from the workbook's `Summary` sheet:

- **core** — the independent-source census: Athletic.net and its AthleticLIVE derivative excluded.
- **all sources** — core plus the Athletic.net ids that other sources publish. The ids are
  *enrichment keys, never dereferenced*: the export path spent zero Athletic.net requests.

## 1. How many Midwest Class-of-2027 athletes each source discovers

Per-jurisdiction cohort and core share, from the `Coverage` sheet (`Athletes` = all sources,
`Athletes (core)` = without Athletic.net-derived rows):

| State | Athletes | Core | Core share | Boys | Girls | Schools | With performance |
|---|---|---|---|---|---|---|---|
| OH | 31,708 | 26,998 | 85% | 16,997 | 14,634 | 1,504 | 0 |
| IL | 26,488 | 19,392 | 73% | 14,878 | 11,594 | 1,889 | 0 |
| WI | 22,074 | 19,281 | 87% | 12,014 | 10,031 | 1,027 | 7,503 |
| MI | 22,073 | 16,176 | 73% | 12,855 | 9,218 | 1,493 | 0 |
| IN | 16,931 | 11,909 | 70% | 9,380 | 7,446 | 1,108 | 0 |
| MO | 15,168 | 13,218 | 87% | 8,576 | 6,590 | 1,038 | 0 |
| MN | 14,420 | 11,261 | 78% | 7,957 | 6,463 | 1,016 | 0 |
| IA | 14,076 | 8,738 | 62% | 7,906 | 6,108 | 913 | 0 |
| KS | 10,077 | 9,252 | 91% | 5,728 | 4,348 | 836 | 0 |
| NE | 9,200 | 5,921 | 64% | 5,067 | 4,127 | 678 | 0 |
| SD | 4,868 | 2,669 | 54% | 2,774 | 2,094 | 427 | 0 |
| ND | 3,004 | 2,043 | 68% | 1,623 | 1,381 | 339 | 0 |
| **Midwest** | **190,087** | **146,858** | **77%** | **105,755** | **84,034** | **12,268** | **7,503** |

## 2. How many unique athletes remain after deterministic reconciliation

190,087 in the twelve states; the store's canonical athletes reconcile one for one
(`Run Metrics` → every `Reconciled counter` row reads `reconciled`). Nationally the cohort is
625,899 (all sources) / 582,670 (core).

## 3. What percentage have Athletic.net profiles

**93,619** athletes carry an Athletic.net profile URL obtained **without a single Athletic.net
request** (`Summary`: "Athletes whose Athletic.net profile URL is known without an Athletic.net
request"). In the twelve Midwest states the per-state `Athletic.net URL` counts are the `Q` column:
IL 10,740 · WI 5,185 · IN 3,188 · IA 2,820 · MO 5,552 · NE 3,066 · KS 1,937 · MI 9,148 · MN 5,315 ·
ND 985 · OH 9,891 · SD 1,442.

## 4. What percentage have an independently corroborating source

All-sources scope: **59,269** athletes carry two or more independent sources (`Summary`:
"Class of 2027, multiple sources"). Core scope reads 0 by construction — the core *is* the
independent set, so a second source is what promotes a row into it.

## 5. What percentage have a school with an identified track/XC coach

**64,370** cohort athletes (all sources; 57,840 core) sit at a school with an identified
track/XC coach — `Summary`: "Class of 2027, coach identified".

## 6. What percentage have a public professional coach contact

**46,224** cohort athletes (all sources; 41,404 core) have a coach professional email.
Store-wide: 28,978 coach/AD rows, **8,985** of them carrying a professional address — consumer
mailboxes are withheld at consolidation (`model::professional_email`).

## 7. How many Athletic.net requests are avoided

**All of them.** The export path's HTTP cache holds 37,765 responses across 15 hosts and no
`athletic.net` entry; meet ids (9,593), athlete ids (93,619) and profile URLs arrive as ids other
sources publish. A whole-meet route exists (two requests per meet) and is the one planned
Athletic.net surface, run deliberately rather than as discovery.

## 8. Which states/providers account for remaining coverage gaps

From the `Coverage` sheet's gap rows (cohort athletes lacking each dimension):

| State | missing performance history | missing coach | missing profile | missing PR support |
|---|---|---|---|---|
| OH | 31,708 | 28,639 | 1,716 | — |
| IL | 26,488 | 18,288 | 486 | — |
| WI | 14,571 | 5,485 | 4,153 | 90 |
| MI | 22,073 | 20,046 | 106 | — |
| IN | 16,931 | 13,796 | 2,173 | — |
| MO | 15,168 | 13,380 | 47 | — |
| MN | 14,420 | 3,212 | 75 | — |
| IA | 14,076 | 13,971 | 2,729 | — |
| KS | 10,077 | 3,822 | 189 | — |
| NE | 9,200 | 2,804 | 1,332 | — |
| SD | 4,868 | 4,585 | 195 | — |
| ND | 3,004 | 1,087 | 1 | — |

`missing_performance_history` is the dominant class: WI is the only state whose store carries
performances (7,503 athletes), because the timer/result adapters have only been walked for
Wisconsin. The Ohio and Michigan coach gaps are the largest single deficits after that.

## 9. Which five to ten production adapters give the best coverage/engineering ratio

Ranked by marginal verified coverage per unit of work already spent (evidence:
`synthesis/03-adapter-ranking.md` plus the measured store):

1. **MileSplit state rosters/grade pages** — the cohort backbone in every state.
2. **AthleticLIVE timer index** — publishes the Athletic.net athlete id in the `ani` field; the
   source of most profile URLs.
3. **WIAA / KSHSAA / MSHSL / NDHSAA-NSAA association directories** — coach and AD identity.
4. **State association result/tournament surfaces** (IHSA tournament decode, OHSAA integration).
5. **Athletic.net whole-meet route** — two requests per meet for full result blocks; the single
   cheapest way to close `missing_performance_history`.
6. **MileSplit meet index + `milesplit_results`** — meet discovery and result pull without
   Athletic.net.
7. **DirectAthletics state/team indexes** (IL, KS) — school aliases and meet ids.
8. **PrimeTime / Hy-Tek / RaceDay / compiled-format timers** — byte-level parsers, fuzzed clean.

## Caveats carried with these numbers

- The workbook records a snapshot, not a season-long census; `observed_on` = 2026-09-22.
- The coach lanes for OH, KS, SD and a second MN pass are still running; MN alone contributed
  1,131 rows in its first pass and matched 454 schools store-wide, so count 5/6 will rise.
- Meet enumeration (`meets --all-states --year 2026`) is in flight and does not change cohort counts.
- `missing_performance_history` closes only when the whole-meet pulls run; they are the next
  scheduled store writes.
