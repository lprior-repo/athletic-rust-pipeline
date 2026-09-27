# Midwest source research: evidence index and dated corrections

Historical 2026-09-18–23 research, first over twelve Midwest states and later broader national
sources. **Not the current run, architecture, source policy or delivery plan.** Current authority:
[architecture](../../ARCHITECTURE.md), [adapter guide](../../SOURCE_ADAPTER_GUIDE.md) and
[49-jurisdiction delivery plan](../../docs/NATIONAL-CENSUS-PLAN.md). Old 50/51/52-location outputs
have different scope/unknown buckets; none changes today's denominator or certifies a fresh run.

The corpus originated in an external research workspace; retained files are now under this directory.
Historical absolute paths identify the original artifact, not an available current input. Check the
actual retained bytes before reproducing a claim. No source captures/data are removed by this cleanup.

## Evidence owners

| Record | Owns |
|---|---|
| [Individual reports](research/midwest/) 01–29 and 31–47 | Source-specific observations, negative findings, timestamps, capture locators; 47 is the coach provenance audit |
| [Acceptance measurements](synthesis/01-acceptance-answers.md) | Nine measured questions for the 2026-09-21/22 corpus; source-ID overlap is not automatically independent identity corroboration |
| [Historical source matrix](synthesis/05-source-matrix.md) | Cross-source/jurisdiction comparison and conflicting-count/retraction ledger; not current registry applicability |
| [Compliance record](synthesis/05-compliance-and-risks.md) | Original refusals/disclosures and 2026-09-20 request-cache ledger |
| [HAR endpoint baseline](synthesis/atn-endpoint-groundtruth.md) | 2026-09-18 request/response field inventory |
| [Live closure captures](synthesis/08-gap-closure-2026-09-20.md) | Later response inventories, token-redacted digests and endpoint corrections |
| [Measured census](synthesis/10-measured-census.md) | 2026-09-22 generated census/coach tables |
| [Acceptance audit](synthesis/12-acceptance-audit-2026-09-22.md) | Audit of the earlier sealed workbook and its per-state gaps |
| [Routed-ingest/seal evidence](synthesis/14-routed-ingest-and-seal-2026-09-23.md) | Later run/seal, bounded sampled verification and known limitations |
| [Coach audit](research/midwest/47-coach-fragment-provenance-audit.md), [first merge](data/coach-merge-report.md), [union merge](out/merge-report-2026-09-23.md), [Rust gate](out/coach-gate-rust-table.md) | Distinct coach generations and row-level verification, not competing current procedures |
| [Engineering run](pipeline/WAVE-4-5-REPORT.md) | Dated gate/parity execution; not today's gate result |

`data/`, `reports/`, `out/`, `pipeline/`, `targets/` and the reports' `evidence/` retain underlying
products. National lane reports live in [../sources/](../sources/); the current research index and
citation contract is [../README.md](../README.md). Bracketed `[NN]` means that numbered local source
report; `[lane: name]` means its national lane report. `[INFERENCE]`/`[UNVERIFIED]` remain non-measurements.

`census-service qa-reports --research research/midwest-source-program` checks the 45 source reports
01–29/31–46 and this index; report 47 is a separately shaped audit. The retained mirror is incomplete:
the 2026-09-27 check found missing capture directories for 31, 32, 33, 35, 36, 37, 40, 41, 42, 43,
45 and 46. It correctly refuses complete evidence qualification. Do not create empty directories
or silently weaken the check; locate the original public captures if full reproduction is required.

## Retained facts from removed summaries

This section holds only unique evidence/corrections formerly stranded in competing plans, status
reports and synthesis digests. Detailed measurements already owned above are linked, not copied.

### Research baseline and attribution corrections

- The initial executive summary recorded 30 research assignments, approximately 2,000 HTTP requests
  across approximately 120 hosts, and two 2026-09-18 HARs (`www.athletic.net.har`, 16.8 MB;
  `www.athletic.net2.har`, 7.9 MB). Its first build reported 707,933 athlete records, 183,875 Co2027,
  93,619 Athletic.net athlete IDs and 9,593 meet IDs, with zero Athletic.net requests by that build.
  These are historical rows/IDs, not a reverified unique-person count.
- The measured twelve-state boys-outdoor sweep was 2,728 requests / 40,695 athletes. Report 27's
  per-state figures summed to 41,145 while its old text printed 46,145; the +5,000 inconsistency was
  identified in the removed Pareto digest. Do not average these distinct attributions. The national
  baseline had 142,705 athletes, 95 events and 4,256 receipts (4,233 modeled requests).
- The old state-meet grade-oracle aggregate was 3,993 rows, 9.8% of 40,695. The follow-up reports added
  evidence but did **not** recompute that aggregate. It is not interchangeable with the later
  two-namespace rate. The prior model's roughly 3,920–3,960 profile-class requests plus 190 meet
  lookups concerned that narrow oracle set, not the census's later 18,424-request spend.
- The Pareto model estimated 6,302 athletes / 15.5% in coach-less MI/IN/MO on its 40,695 baseline;
  later census measurements use a different cohort and add KS. The source-ID overlap model also
  does not establish that all resulting canonical rows are the same person.
- The retired adapter ranking's IHSA “42/56” claim had no supporting artifact. OHSAA owned the
  56-cell observation (33 named, all 33 mailto); IHSA's 14-school staff sample had 49/49 TF/XC coach
  rows with `HasEmail`. Read reports 13 and 19 rather than transferring a percentage across states.
- Historical meet-seed recount: 9,844 rows total, 9,747 nonempty linked IDs, one `id=0` test meet,
  hence 9,746 usable—not 9,747 real meets. Filtered IL/MI counts were 2,128/1,421, versus unfiltered
  2,159/1,433. The 20,254-row harvest had 18,112 `has_results` rows and 2,142 without results;
  sentinel AN IDs included `-1` ×118, blank ×109 and `0` ×6. These are different files/denominators.
- AthleticLIVE discrepancies remain explicit: tenant sum 153,524 versus wildcard result 153,774;
  text-match versus exact-keyword counts ND 5,222/875, SD 3,188/1,869 and NY 9,664/6,917. A text
  match is not an exact state census. The matrix's attribution to index churn was an interpretation,
  not a reconciled 250-document proof.
- Report 18's grade correction is retained in its own corrections log: 1,861 Co2027 via `year=JR`
  (1,155+706), versus 1,678 `year=SR` for Co2026 on that artifact. Report 09, not the old coach digest,
  evidences MN coach emails on the sampled endpoint. NSAA WordPress refusal and successful forms-host
  access are different surfaces. Bound's empty SD results are a publication gap, not proof every
  jurisdiction has no Bound data. Current policy independently controls whether access is permitted.

### Earlier versus later measured render

The removed `.py.md` generator render and retained [measured census](synthesis/10-measured-census.md)
were different generations, not duplicate byte-for-byte outputs:

| Quantity | Earlier render | Retained later render |
|---|---:|---:|
| Schools | 32,096 | 32,100 |
| Coaches / with email | 30,243 / 9,730 | 31,488 / 10,671 |
| Exported Co2027 rows | 213,215 | 625,899 |
| Rows with AN / MileSplit / both IDs | 93,619 / 165,663 / 59,269 | 93,619 / 578,347 / 59,269 |
| WI schools / coach-email-linked rows | 1,027 / 15,483 | 1,031 / 16,218 |
| Recruiting coach-linked / coach email / AD email | 41,874 / 32,011 / 40,554 | 81,990 / 55,964 / 71,724 |
| States in coach-coverage table | 10 | 23 |

A separate 2026-09-22 store status reported 213,198 Co2027, whereas later artifacts report 625,899
and the twelve-state filter 190,087. These generation differences remain unreconciled here; no
current seal or completeness inference follows from choosing the largest number.

### Store/index and review incident — 2026-09-22

The removed live-status report recorded the first index pass in that store, 177 seconds:

| Plane | Before | After |
|---|---:|---:|
| Observations | 2,129,046 | 2,129,051 |
| Source identities | 0 | 1,116,961 |
| Conflicts | 0 | 1,361 |
| Review cases | 0 | 27,967 |
| Coverage rows / snapshots | 0 / 0 | 210 / 1 |

Schools/teams/coaches/athletes/meets were unchanged at 62,063 / 138,995 / 32,264 / 1,565,219 / 12,209.
Footprint: 7.6 GB root, 1.8 GB entity bytes. The all-source projection reported 879,587 athletes /
213,198 Co2027, versus core 743,739 / 169,969; 13,483 schools and 27,580 coaches attached.
199,996 of the former cohort had a profile URL; 59,269 had the report's two-namespace attribution.

At 07:09–07:11 UTC approximately 45 MileSplit indexes succeeded (2,454 cache artifacts), then all
582 roster attempts failed. The report recorded first-refusal source-access persistence, six-hour
cooldown and `blocked_skipped`. Same-day correction: the restriction cleared; the WI limit-20 probe
skipped 597 journaled rosters with zero requests, while AL limit-3 refresh made four requests with
zero errors. Consolidation changed schools 13,483→13,486, teams 75,940→75,946 and athletes
879,587→879,664. This was a recorded transient response, not evidence of a permanent ban or authority
to bypass one. The old commands are historical, not today's routing instructions.

The old single-server Qwen3.6-35B-A3B review pass measured roughly 16 s/case and 13 tokens/s.
Families: 26,608 cohort-evidence cases not askable, 917 mailbox-withheld cases, 442 meet venues,
0 school jurisdictions, 1,249 athlete conflicts and 112 school conflicts. Recorded batches were
`asked=5 accepted=2 rejected=0 insufficient=3` and the earlier venue rule's
`asked=3 accepted=0 rejected=3`; approximately 429 placements remained. This does not qualify the
current required dual-server identity lane.

### Retractions, follow-up limits and publication caveats

- Captured “browser-grade header” successes do not authorize spoofing/direct HTTP as the current
  Athletic.net transport. Preserve them as historical observations, not a “no bypass anywhere”
  certification. Current policy requires the headed lane. Later re-verification superseded the
  earlier blanket-403 inference, but not policy. Never persist token/cookie secrets.
- IA/SD Bound directory claims and the MHSAA bulk API recipe were retracted on robots grounds.
  MSHSAA had a recorded universe fetch before the later merged-group disallow-all interpretation;
  retain that disclosure. PrimeTime/Karmarush terms refusal stays in the compliance ledger.
- The follow-up corrections live in reports 31–46: report 32 corrected “44 locked rows” to 11 rows /
  55 masked cells; report 33 measured a rolling 7,500-URL sitemap with 44–96% turnover in under ten
  hours, not exhaustive enumeration; report 34 measured a WI/MN boys complement but could not validate
  girls against the boys corpus; report 46 distinguished 3,627 affected athlete rows (2.542%) from
  10,943 numeric token occurrences. Preserve their sampling and unresolved recurrence/OCR/join limits.
- The coach union “missing rows” alarm was deduplication, not established data loss. The retained
  union report owns the exact keep/duplicate/reject counts; the frozen CSV was already merged.
  Reasserting verified rows did not delete prior rows, so the store remained a superset. The coach
  audit retains the 241 published rows lacking a verified counterpart: OH128/IA47/WI34/SD26/MI4/IN2.
- Historical seven-host overlap between the compiled coach artifact and live adapters meant repeated
  evidence dates, not independent corroboration. Report 47 owns the measured interaction. Current
  source reconciliation must not reuse the obsolete proposal to preserve MI/IA/SD bypass fragments.
- Early seal `7b41a26465e24c2e3c21de1b22f3e66a4e5a139b815793c5a162497002fc2723` was replaced at
  2026-09-22 11:53 by `c63f7fb4f38f55167fc3019e722bff122bbac2fc156410552935d786c169da21` after
  coach union/import. The earlier workbook was no longer retained; its audit tables are not a
  readback of the replacement. The later 2026-09-23 seal and its sampled—not complete—readback are
  owned by the routed-ingest evidence above. None proves the fresh run's full independent oracle.

Historical coach CSV header:
`school,city,state,sport,role,coach_name,public_professional_email,ad_name,ad_email,source_url,last_observed`.
School-role/date/source provenance, no guessed values, and separate denied/private-contact classes
remain meaningful; old work assignments, refresh schedules and competing build orders do not.
