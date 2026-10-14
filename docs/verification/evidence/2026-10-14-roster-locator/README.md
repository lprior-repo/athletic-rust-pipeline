# Detached roster locator evidence

Start with [REPORT.md](REPORT.md). This package is additive to the durable-capture checkpoint and preserves historical evidence rather than editing it.

- `candidate-first/`: exact first formatted 22-file overlay and the historical 73-test result
- `candidate-final/`: exact overlay after the two LR01/LP04 regression additions; production bytes unchanged
- `inputs/baseline-probe*/`: separately executed old-production test-only probes, original raw outputs, source and logs
- `inputs/adopted-contract/`: adopted narrow scope and prior contract review
- `raw-logs/`, execution ledgers and Root receipts: terminal execution facts and explicitly attributed numeric exits
- `review-initial/` and `review/`: preserved finding history and final bounded source/suite review
- `traceability-matrix.jsonl`: LP01–LP09 test/source/result mapping and limitations

Run `python3 verify_bundle.py` from any directory to validate the portable package's own artifact pins, source snapshots, JSON, log identities, selected counts and authored relative links. This is evidence-integrity validation only. It does not build the repository or replace active-context truth-serum.

The original copied input/reviewer pin files retain their historical path conventions. Use the package-level `artifact-pins.sha256` for this portable layout; do not assume every historical manifest was authored relative to its copied directory. Source origin and reviewer coverage are reconciled explicitly against the package's first/final snapshot manifests.

The final local source commit is f4506762506a4c2019b2b610f4554f3b77d005a1. commit-coverage.json independently matches all 22 changed paths to the tested/reviewed final snapshot; the worktree was observed clean. Exact historical execution commands preserve their actual original paths. This is not a claim that those machine-local commands run unchanged on another computer.
