# 10. Measured census — what the pipeline actually collected

Generated from `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/midwest-census/out` on 2026-09-22. Every figure below is read from `out/report.json`, `out/athletes.jsonl`, `out/meets.jsonl` or the CSVs in `data/`; research-phase estimates stay in `synthesis/01-acceptance-answers.md` and are labelled as estimates there. Where the two disagree, this file is the measurement.

## Totals

| metric | measured |
|---|---|
| schools | 32096 |
| athletes (all grades) | 2373938 |
| Class of 2027 | 625899 |
| Co2027 boys | 352595 |
| Co2027 girls | 272689 |
| Co2027 with a public profile URL | 612697 |
| Co2027 with grade evidence | 625899 |
| Co2027 reachable from two independent sources | 59269 |
| coaches / ADs | 30243 |
| coaches / ADs with a published professional email | 9730 |
| meets | 11007 |
| meets carrying an Athletic.net meet id | 9593 |

## By state

| state | schools | athletes | Co2027 | boys | girls | multi-source | with coach | with coach email |
|---|---|---|---|---|---|---|---|---|
| AK | 173 | 6922 | 1340 | 805 | 535 | 0 | 0 | 0 |
| AL | 560 | 38180 | 9120 | 5247 | 3873 | 0 | 0 | 0 |
| AR | 488 | 47028 | 8113 | 4732 | 3381 | 0 | 0 | 0 |
| AZ | 411 | 30696 | 8582 | 5092 | 3440 | 0 | 2915 | 547 |
| CA | 2024 | 166429 | 52004 | 29709 | 22258 | 0 | 4644 | 4187 |
| CO | 429 | 32626 | 10860 | 6244 | 4611 | 0 | 3422 | 0 |
| CT | 251 | 22109 | 6674 | 3641 | 3033 | 0 | 0 | 0 |
| DC | 68 | 331 | 126 | 55 | 71 | 0 | 64 | 34 |
| DE | 82 | 6496 | 2090 | 1139 | 948 | 0 | 0 | 0 |
| FL | 1024 | 61177 | 18212 | 10571 | 7639 | 0 | 61 | 61 |
| GA | 707 | 68817 | 20318 | 11634 | 8681 | 0 | 3918 | 0 |
| HI | 111 | 6178 | 1619 | 952 | 667 | 0 | 0 | 0 |
| IA | 913 | 62201 | 14076 | 7906 | 6108 | 2820 | 1785 | 1402 |
| ID | 199 | 21597 | 3793 | 2184 | 1609 | 0 | 0 | 0 |
| IL | 1889 | 92973 | 26488 | 14878 | 11594 | 10740 | 8170 | 7958 |
| IN | 1108 | 54475 | 16931 | 9380 | 7446 | 3188 | 2980 | 2800 |
| KS | 836 | 33628 | 10077 | 5728 | 4348 | 1937 | 946 | 636 |
| KY | 438 | 31773 | 6637 | 3729 | 2907 | 0 | 0 | 0 |
| LA | 582 | 26583 | 7357 | 4021 | 3336 | 0 | 0 | 0 |
| MA | 462 | 40203 | 12037 | 6679 | 5301 | 0 | 0 | 0 |
| MD | 401 | 27712 | 9141 | 5181 | 3944 | 0 | 0 | 0 |
| ME | 160 | 9022 | 3331 | 1831 | 1486 | 0 | 0 | 0 |
| MI | 1493 | 114289 | 22073 | 12855 | 9218 | 9148 | 1531 | 1188 |
| MN | 1016 | 64275 | 14420 | 7957 | 6463 | 5315 | 11147 | 10326 |
| MO | 1038 | 50830 | 15168 | 8576 | 6590 | 5552 | 1097 | 918 |
| MS | 460 | 30148 | 7139 | 4115 | 3024 | 0 | 0 | 0 |
| MT | 207 | 15663 | 2724 | 1589 | 1135 | 0 | 0 | 0 |
| NC | 891 | 61795 | 19400 | 11220 | 8174 | 0 | 0 | 0 |
| ND | 339 | 15138 | 3004 | 1623 | 1381 | 985 | 1909 | 0 |
| NE | 678 | 44983 | 9200 | 5067 | 4127 | 3066 | 6396 | 663 |
| NH | 108 | 7744 | 2293 | 1274 | 1019 | 0 | 0 | 0 |
| NJ | 549 | 58306 | 21361 | 11661 | 9699 | 0 | 1162 | 952 |
| NM | 199 | 14907 | 4185 | 2312 | 1869 | 0 | 0 | 0 |
| NV | 136 | 11514 | 2955 | 1729 | 1226 | 0 | 0 | 0 |
| NY | 1352 | 102462 | 32922 | 17631 | 15272 | 0 | 0 | 0 |
| OH | 1504 | 106136 | 31708 | 16997 | 14634 | 9891 | 4310 | 3997 |
| OK | 769 | 45558 | 8403 | 4899 | 3503 | 0 | 0 | 0 |
| OR | 368 | 36194 | 6878 | 3986 | 2892 | 0 | 2512 | 0 |
| PA | 910 | 83632 | 23084 | 12331 | 10738 | 0 | 0 | 0 |
| RI | 71 | 6025 | 2118 | 1113 | 1003 | 0 | 0 | 0 |
| SC | 452 | 36256 | 9021 | 5055 | 3946 | 0 | 0 | 0 |
| SD | 427 | 22812 | 4868 | 2774 | 2094 | 1442 | 283 | 79 |
| TN | 591 | 30274 | 9514 | 5327 | 4187 | 0 | 2257 | 2257 |
| TX | 2408 | 252384 | 63507 | 36632 | 26861 | 0 | 1900 | 1446 |
| UNKNOWN | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| UT | 171 | 16102 | 5209 | 2947 | 2262 | 0 | 2767 | 2591 |
| VA | 602 | 56490 | 16619 | 9494 | 7078 | 0 | 0 | 0 |
| VT | 117 | 4960 | 889 | 473 | 416 | 0 | 0 | 0 |
| WA | 647 | 55372 | 11774 | 7097 | 4677 | 0 | 0 | 0 |
| WI | 1027 | 125844 | 22074 | 12014 | 10031 | 5185 | 16561 | 15483 |
| WV | 169 | 10228 | 2295 | 1294 | 1001 | 0 | 0 | 0 |
| WY | 81 | 6461 | 2168 | 1215 | 953 | 0 | 0 | 0 |

## Class of 2027 by sport

| bucket | athletes |
|---|---|
| cross_country_only | 63366 |
| indoor_only | 25190 |
| multi_sport | 193539 |
| none | 100833 |
| outdoor_only | 242971 |

## Athletic.net identities obtained without querying Athletic.net

- **169,230 distinct Athletic.net athlete ids** were published by non-Athletic.net sources (AthleticLIVE timer rows that carry the `ani` field; MileSplit pages where the Athletic.net link is embedded). Each id yields the profile URL by composition: `https://www.athletic.net/athlete/<id>/track-and-field`.
- **9,566 distinct Athletic.net meet ids** were published by the timer meet index (`data/athleticnet-meet-seeds.csv`), so a targeted meet pull needs no search.
- 72,544 canonical athletes are corroborated by more than one independent source (different namespaces on the same canonical id).
- Of the 213,215 Co2027 rows in `data/canonical-athletes-co2027.csv`: 93,619 carry an Athletic.net athlete id, 165,663 carry a MileSplit athlete id, 59,269 carry both kinds of identity.

## Coach coverage

| state | coach rows | with email |
|---|---|---|
| IA | 53 | 4 |
| IL | 15102 | 3235 |
| KS | 526 | 523 |
| MI | 6 | 6 |
| MN | 6596 | 2333 |
| ND | 922 | 0 |
| NE | 1772 | 0 |
| OH | 28 | 23 |
| SD | 25 | 5 |
| WI | 2550 | 2155 |

- Recruiting projection (`data/recruiting-co2027.csv`): 213,215 Co2027 athletes, 41,874 linked to a named head track/XC coach, 32,011 with a head-coach professional email, 40,554 with an athletic-director email.

## Meets by state and provider

| state | meets |
|---|---|
| ?? | 442 |
| IA | 1051 |
| IL | 1994 |
| IN | 381 |
| KS | 181 |
| MI | 1429 |
| MN | 1054 |
| MO | 569 |
| ND | 161 |
| NE | 589 |
| OH | 1232 |
| SD | 364 |
| WI | 1560 |

99 timer/tenant providers published the meet universe; the largest ten:

| provider | meets |
|---|---|
| timer_meet:live_results | 9705 |
| timer_meet:athleticlive | 2432 |
| timer_meet:wayzata | 1264 |
| timer_meet:michiana | 440 |
| timer_meet:dakota | 421 |
| timer_meet:iptt | 403 |
| timer_meet:palatine | 349 |
| timer_meet:blacksquirrel | 345 |
| timer_meet:heros | 329 |
| timer_meet:aatiming | 311 |

## Measured answers to the acceptance questions

| question | measured answer |
|---|---|
| Q1 how many Co2027 athletes each source discovers | see the by-state table; AthleticLIVE and MileSplit contribute 2,373,938 canonical athletes across all grades, 625,899 of them Co2027 |
| Q2 unique athletes after reconciliation | 2,373,938 canonical athletes minted from (school, name, class, gender) — no vendor id required |
| Q3 share with an Athletic.net profile | 93,619 of 213,215 Co2027 rows carry an Athletic.net id (43.9 %) |
| Q4 share with independent corroboration | 72,544 athletes carry identities from two independent namespaces |
| Q5 share with an identified coach | 41,874 of 213,215 Co2027 athletes (19.6 %) |
| Q6 share with a public professional coach email | 32,011 of 213,215 (15.0 %); AD email 40,554 (19.0 %) |
| Q7 Athletic.net requests avoided | 169,230 athlete profiles and 9,566 meet resources are addressable by id without any Athletic.net search or enumeration |
| Q8 remaining gaps | ND and SD MileSplit team indexes answered HTTP 403 and were not bypassed; MO/IN coach contact remains unpublished; see `synthesis/05-compliance-and-risks.md` |
| Q9 best adapters | `data/adapter-ranking.csv`, now with measured yields substituted where the adapter was run |

## Limits and honest gaps

- The measurement covers what the runbooks in `tools/run_pipeline.sh` executed on 2026-09-20; it is a snapshot, not a season-long census.
- Team rosters only surface athletes whose school publishes a roster on the source used; athletes on teams with no published roster are invisible to this pass.
- `events` and `performances` tables are intentionally empty in this store: results acquisition stays with the production pipeline; this store carries identity, school, coach and meet facts.

## Reproduce

```bash
# 1. crawl + adapters + merge + census + exports (idempotent, journal-resumed)
tools/run_pipeline.sh
# 2. rebuild this document from the fresh snapshots
python3 tools/make_census_doc.py
```
