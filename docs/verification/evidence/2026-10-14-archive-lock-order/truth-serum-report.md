# Artifact and claim audit

STATUS: UNVERIFIED

Local artifact-integrity and claim checks were executed in this evidence-packaging worker context. They verify:

- All 16 formatted source hashes match both the copied source bytes and exact committed git object bytes.
- The commit's changed path set equals the 16-file snapshot set; the commit/tree are read directly from git.
- Raw nextest output has exactly 96 PASS rows and its summary reports 96 passed, 3200 skipped. Every one of the 16 named tests has one exact PASS row, a matching source function and the correct source hash.
- The isolated child's direct selection is explicitly qualified. Only the passing parent proves invocation with invalid-capacity environment and nonzero child test selection.
- All five Root commands (initial format write, focused tests, lint, check and format check) have terminal logs and exit-0 executor attestations. The receipt is labeled as an attestation, not a raw tool transcript.
- Both historical and corrected reviews/patches are retained. Current review hash equals the corrected patch hash. All four findings have canonical fixed_with_evidence dispositions.
- Bundled JSON is parseable, all inventoried files are nonempty, and all artifact hashes verify.
- Pending gate language was removed only after terminal results and source commit were provided; not-run limits remain explicit.

See ARTIFACT-AUDIT.json for machine-readable checks, SHA256SUMS for every file hash, and raw/ for exact command evidence. The check script used Python standard-library hashing/JSON/log parsing and read-only git object queries. No build, test, format, lint, benchmark or live edit was performed by this worker.

A search of recovered skills and `command -v truth-serum` found no available dedicated truth-serum skill/executable. Therefore that specialized audit is NOT RUN and this report does not issue formal bundle approval. Root owns final evidence acceptance. No correctness, performance, v2, native-fault or release claim is added by packaging.
