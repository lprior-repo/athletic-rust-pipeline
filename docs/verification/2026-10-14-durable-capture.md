# 2026-10-14 durable capture integration checkpoint

Technical qualification: **UNVERIFIED**. This is an explicitly in-progress checkpoint for the owner-authorized non-draft PR update. Historical failures and pending reruns remain visible; publishing source does not certify release, production, a completed census or a publication.

Base commit: `5111a603cc1d945e2da3f85c00d149796552e43e`. Branch: `implementation/durable-capture-integration-20261014`. Source captured: `2026-10-14T07:07:43.794833+00:00`. Exact dirty-source scope: 237 files (146 tracked changes, 91 additions). [Source ledger](evidence/2026-10-14-durable-capture/source-files.json) and [SHA-256 manifest](evidence/2026-10-14-durable-capture/source-pins.sha256) describe the source independently of a later commit ID.

- [Consolidated assurance and limits](evidence/2026-10-14-durable-capture/assurance-bundle.md)
- [Exact commands and environment](evidence/2026-10-14-durable-capture/commands.md)
- [Chronological verification ledger](evidence/2026-10-14-durable-capture/verification-ledger.jsonl), [raw-log hashes](evidence/2026-10-14-durable-capture/raw-log-ledger.json) and [selected test names](evidence/2026-10-14-durable-capture/selected-test-results.json)
- [Spec acceptance mapping](evidence/2026-10-14-durable-capture/acceptance-map.jsonl)
- [Per-file exact review coverage](evidence/2026-10-14-durable-capture/review-coverage.json) and [finding dispositions](evidence/2026-10-14-durable-capture/findings.jsonl)
- [Technical evidence decision](evidence/2026-10-14-durable-capture/final-evidence-decision.md)

Lifecycle 1/2 historical meaning and lifecycle 3 enrollment-only restrictions are preserved. Lifecycle 4 is explicitly initialized fresh capture qualification only. The proposal-only lifecycle-5 lost-backing design is not part of this implementation.

Historical core: 30/30 pass. Historical composed lane: 143/144 pass, one stale service unknown-outcome oracle failed. Later source repairs and additional regressions must be rerun. Serial all-target compilation passed before the latest boxing repairs. The subsequent fmt-check passed, while lint failed with 86 service size diagnostics. Both narrow boxing repairs are applied and the final fmt-write passed. The expanded composed-third rerun is running at this freeze. Its tests and final lint/check reruns remain pending; Root will append actual terminal results.

Prepared lost backing, the 2 GiB acquisition inventory/writer-capacity mismatch, native fault/dispatch/archive/restart evidence, whole-workload resource measurement and full-spec recruiting-product qualification remain explicit gaps. None is waived by this checkpoint.
