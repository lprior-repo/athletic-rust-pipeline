# Scoped parser-envelope assurance bundle

STATUS: REJECTED

This is a detached evidence proposal. Root owns final integration and acceptance. The [requirement mapping](ACCEPTANCE-MAPPING.md), [machine-readable map](acceptance-map.json), [gate ledger](GATE-STATUS.json), [source pins](SOURCE-PINS.json), [API states](API-PROBE-STATUS.json) and [limits](LIMITS.md) are integral to this bundle.

## Review findings and disposition

| Finding | Recorded disposition | Evidence class and remaining limitation |
|---|---|---|
| PE-1 capture/provenance recombination | `fixed_with_evidence` in preserved review | Static source and selected real-producer regressions PASS; compiler/API named oracles PASS |
| PE-2 caller verdict/resource ownership | `fixed_with_evidence` in preserved review | Mapped behavior regressions PASS; compiler/API named oracles PASS |
| PE-3 synthetic historical authority | `fixed_with_evidence` in preserved review | Static source plus sixth regression PASS; production-absence compiler probes PASS |
| PE-4 same-variant arbitrary internal context | `owner_approved_no_action` in preserved review | Root expressly retained variant-only scope; exact binding belongs only to traced production callers |
| PE-COMPILE-1 seven empty-journal collection mismatches | `fixed_with_evidence` limited to detached/source correction | All original E0308 diagnostics preserved; exact seven changes pinned; fifth focused run including all six tests passed |
| PE-EXEC-2 earlier compiler SIGKILL attempts | `fixed_with_evidence` for scoped execution only | Fifth focused command completed 107/107; all earlier failures retained. This resolves the blocked selected test result, not the unverified termination causes |
| PE-EXEC-PENDING executable focused/API acceptance | `fixed_with_evidence` | 107/107 focused tests, all 8 API oracles and clean-source basic checks passed |
| PE-SIZE-14 strict callable-size gate | `blocker` | 14 functions >60 logical lines; exact sites in STATIC-GATE-RESULTS.json; disjoint/unchanged paths do not waive failure |
| PE-SEAMS-3 module-seam gate | `blocker` | 3 installation-to-bootstrap edges remain forbidden; zero crate-seam violations |
| PE-PINS-PENDING final source commit | `fixed_with_evidence` | Source commit `771072a1013017a4f8b4993751f801df16f789a2` and tree are verified; all 38 paths match their snapshot bytes |
| PE-AUDIT-MISSING active-context truth-serum | `blocker` | Required skill/tool unavailable; manual artifact integrity audit is not substituted |

The historical review's `fixed_with_evidence` dispositions explicitly state static evidence class and are not upgraded to runtime proof. No review finding is silently omitted. No behavioral waiver or scope expansion is inferred.

## Commands and raw evidence

`raw/root-command-receipts.json` retains exact Root-observed invocations, setup, session IDs and terminal outer statuses. Completed logs retain the actual outputs. First and second format writes and subsequent format-check, source lint and all-target check passed. First focused invocation failed with seven type mismatches; second, third and fourth failed with SIGKILL. The third invocation's environment debug override was ineffective: its raw compiler command still used `debuginfo=line-tables-only`. The fourth invocation's explicit forwarded `--config 'profile.test.debug=0'` took effect (no line-tables-only argument; `strip=debuginfo` present), but compilation still failed. The fifth invocation passed 107/107 (1 slow), with 3195 skipped, under the same selector and explicit debug setting plus `RUSTFLAGS='-Z threads=1'`. Test duration 117.490s; total Moon 14m28.946s; outer exit 0. [GATE-STATUS.json](GATE-STATUS.json) is the current command index.

Prepared test/probe commands in historical documents are plans, not evidence of execution. All six new tests and the broader selected producer/persistence/resource/buffer families are reconciled to 107 distinct actual PASS records in `TEST-RESULTS.json`. All eight API probe results are reconciled to their exact intended outcomes and 22 named negative expression diagnostics; generic failure is not counted as success.

## Proof and wider release evidence

No formal proof, mutation/fuzz/benchmark campaign, native fault lane or release acceptance was executed by this assembler. Existing parser semantics and parser budgets are not rewritten in this slice. Repository release obligations remain distinct. The repository's owner-retired mandatory mutation campaign is not reinstated here, but neither is mutation resistance claimed as executed evidence.

## Artifact audit

The active assembler checks scoped hashes, file existence, JSON parseability, proposed source delta coverage, preservation of failed logs and status distinctions. Results are retained in `ARTIFACT-AUDIT.json`. This audit does not run repository commands or replace the unavailable truth-serum skill. See [truth-serum-report.md](truth-serum-report.md) and [final-evidence-decision.md](final-evidence-decision.md).

## Terminal static results

Strict-size scan and module-seam gates failed; comments and panic-extraction passed. STATIC-GATE-RESULTS.md and STATIC-GATE-RESULTS.json retain all exact findings, disjointness checks and unchanged-file checks. These failures block full acceptance despite the focused/API/basic-gate passes. The original failed prerequisite build and interrupted comments attempt are also retained.
