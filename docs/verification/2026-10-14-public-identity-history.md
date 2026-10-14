# Public identity history foundation — 2026-10-14

R3 maps to `census-domain/src/public_identity_history/` and its library export.
Installation/establishment ordinals define stable public IDs. Immutable binding
claims and accepted-edge references support deterministic merges, precise edge
revocation, surviving-component checks, explicit splits and ambiguous predecessor
aliases. This is a pure candidate history; it is not an accepted or committed
store capability, and no existing athlete ID is rewritten.

Independent review repaired redundant-edge loss, component allocation before
admission, and lost correction history across later splits/merges. Bounded replay
checks head, sequence, payload digest, references, graph work and allocations.

Cloud validation, CI unset and MOON_RUST_BUILD_JOBS=1:
`tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain --lib -E 'test(public_identity_history::)' --no-capture --no-tests fail`
- Initial compilation FAILED: six errors from ambiguous test HistoryError imports;
  no tests ran. Fixed the explicit test import, without changing production behavior.
- Final: 50 PASS, 0 FAIL, 582 skipped; Nextest
  c72cac83-c58f-4e49-bff0-520603f1c935; 0.600s tests, 31.266s task.
- Moon formatting PASS on combined integration tree. All-target/lint PENDING.
- Full release, migration/recovery qualification, production and national run NOT RUN.

OPEN: owner-authenticated correction/binding closure; atomic application, history,
receipt, head and generation persistence; complete frozen ledger and commit-revision
witness; typed publication ID/representative projection; explicit historical adoption.
Unresolved binding reassignment and recurring A→B→A use of an already-seen candidate
remain refused. Caller references/affected-key lists cannot establish authority.
R3 is not closed by this foundation.

Matched-group extension: `EstablishMatchedGroup` accepts one exact SamePerson member
set with one distinct source binding per candidate, allocates one installation/ordinal
public ID, and retains one precisely revocable connection. Already-bound candidates,
duplicate/reused sources, mismatched membership and duplicate authority are refused.
Later split/revocation replay retains unaffected members; public IDs do not depend on
candidate ordering. This remains a pure candidate operation, not source authority.
The same Moon command above now passes 75 tests, 0 failures, 582 skipped (25 added),
Nextest `c4239d44-ce24-4e6a-9922-3d0951ce5d6d`; 0.614s tests / 40.455s task.
Independent review passed. Mixed multiple-new-candidate plus existing-component adoption
remains unsupported; callers must not synthesize repeated authority or SourceBound claims.
