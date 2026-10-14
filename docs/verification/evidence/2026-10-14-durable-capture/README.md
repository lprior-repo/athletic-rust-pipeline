# Portable evidence layout

All raw logs are copies of Root-run output. Commands preserve explicit toolchain homes and inherited-HOME uncertainty. Per-file source SHA-256 entries are relative to the repository root; new files are included. Historical source copies/patches remain in the detached forensic bundle. Review manifests retain their original path spellings and hashes; they identify the source snapshots described by each reviewer, which are not all copied into this portable documentation directory. A hash manifest is provenance, not a passing gate.

Follow source-files.json for the exact publication checkpoint and verification-ledger.jsonl for each lane's source phase. Older logs must not be relabeled as validating post-run edits. Any later results should be appended with their actual command, source pins, status and raw log, preserving every prior red and inconclusive lane.
