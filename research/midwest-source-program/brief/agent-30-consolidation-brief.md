# Agent 30 — Cross-source coverage / Pareto consolidation brief

You are the consolidator for the Midwest TF/XC source exploration. You consume the other 29 reports and
produce the Pareto analysis. You do NOT re-run the state research; you may do a small number of targeted
verification fetches (<= ~25 total) only where two reports conflict and the conflict changes a decision.

## Inputs (read all that exist)

- `brief/mission-brief.md` — rules + evidence standard (follow them).
- `brief/agent-assignments.md` — assignment 30 canonical wording + the acceptance goal.
- `research/midwest/01-*.md` … `29-*.md` — the 29 reports. If one is missing, note it explicitly.
- `synthesis/atn-endpoint-groundtruth.md` — Main-verified Athletic.net endpoint/payload facts (HAR-derived).
- `data/*.csv` — agent-produced datasets (ID catalogs, DAT team index, coverage matrices, coach contacts).
- Repo docs (read-only): `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/{README.md,SCOPE.md,HANDOFF.md}`
  — current pipeline contract: 142,705-athlete retained boys outdoor corpus, 4,256 receipts, 340,238 individual
  results, 57,629 relay rows, 15,724 unresolved; Grade-11 projection is discovery-only, never eligibility.

## Deliverable 1 — `research/midwest/30-cross-source-pareto.md`

Same schema sections as every report (Source / Coverage / Enumeration / Stable identifiers / Athletic.net
leverage / Athlete evidence / Recruiting information / Result evidence / Incremental use / Access
characteristics / Recommendation / Evidence appendix), plus these mandatory sections:

### State coverage matrix
One row per state (12), one column per significant source family, cells = enumerate / seed / results / coaches
with the id types available. Cite the source report per cell (`[06]` style).

### Answers to the nine acceptance questions
Quantitative where the reports support it; otherwise give the bounded estimate + method + confidence:
1. Class-of-2027 athletes discovered per source (per state where known);
2. unique athletes after deterministic reconciliation (dedupe keys named);
3. % with Athletic.net profiles;
4. % with independent corroboration;
5. % with an identified track/XC coach;
6. % with a public professional coach contact;
7. Athletic.net requests avoided through external discovery (show the arithmetic);
8. states/providers accounting for remaining gaps;
9. the 5–10 production adapters with the best verified-coverage/engineering ratio.

### Source-role table
For each source: PRIMARY / ATHLETIC.NET-SEED / RESULT-SOURCE / COACH-DIRECTORY / VALIDATION / DISCOVERY-ONLY /
REJECT, the ids it yields, the Athletic.net requests it avoids, its access class, and its report reference.

### Recommended acquisition flow
Map the plan's preferred flow (official indexes → candidate schools → MileSplit/DAT/result sources → known
Athletic.net ids → targeted Athletic.net → canonical reconciliation) onto the discovered reality, with the
minimum request budget per stage (12-state scope), and call out where the reports contradict the flow.

### Canonical model mapping
For CanonicalSchool → CanonicalCoach → CanonicalAthlete → CanonicalMeet → CanonicalPerformance: which source
supplies which field, which id is the join key, and what remains unjoined.

### Remaining gaps + ranked next actions
Ranked by marginal verified coverage per engineering effort. Explicitly list what a follow-up capture/qualification
session must fetch (e.g. Athletic.net meet responses, AthleticLIVE API, MileSplit grad-year page, XC rankings).

## Deliverable 2 — CSVs in `data/`

- `data/source-coverage-matrix.csv` — `state, source_family, role, enumeration_ids, grade_evidence, result_evidence, coach_evidence, access_class, report_ref, confidence`.
- `data/adapter-ranking.csv` — `rank, adapter, states_covered, athletes_est, coach_est, requests_est, effort_est, ratio_notes, report_refs`.
Schema must be stable and header-named; mark estimates with a `method` or `confidence` column, never blend
observed and inferred values without labelling.

## Rules

- Only information traceable to a report, to the data/ CSVs, to the Main ground-truth file, or to your own
  targeted verification fetch. Unsupported claims marked `[INFERENCE]`/`[UNVERIFIED]`.
- Contradictions between reports: state both, pick the better-evidenced one, and say why.
- Do not re-collect full state datasets; do not modify other reports; write only your report + the two CSVs.
- Record in the Evidence appendix every fetch YOU make beyond reading local files.
