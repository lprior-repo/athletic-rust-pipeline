# ADR-007: A census is sealed, not declared

## Status

Accepted. Implemented: phase ladder and evidence gate (`crates/census-service/src/census/state.rs`,
`.../census/state/evidence.rs`), the `seal` subcommand (`crates/census-service/src/cli/seal.rs`),
tests (`.../census/state/tests.rs`, `.../cli/seal/tests.rs`).

## Context

ADR-006 already requires workbook counts to reconcile against the store before a run may be called
finished, and §17/§70 name the phases and the acceptance items a census must prove. Without an
enforced terminal state the pipeline has two failure modes, and both have a natural pull:

* a report published mid-walk — a partial corpus presented as the census;
* a "complete" label no evidence backs — a claim nothing can be checked against later.

A boolean an operator can set is not a solution to either. What is needed is a state that cannot be
constructed without the evidence, and a refusal that names what is missing.

## Decision

* **One ladder, one value.** `CensusState` *is* the phase: discovering → acquiring → reconciling →
  reviewing → resolving gaps → exporting → complete(sealed). `advance` moves exactly one phase and
  refuses every other transition, so a resumed run cannot jump the phases it has not run.
* **`Complete` is unconstructible without evidence.** The only path into the terminal state is
  `seal(SealEvidence)`, and it refuses unless the census is in the export phase and every §70
  acceptance item is satisfied — naming the item and the numbers behind it.
* **Findings are retained, not fatal.** Gap tallies, conflict counts, exhausted retries and source
  failures travel *inside* the seal. Refusing a seal over a finding would push an operator to hide
  it; only unresolved work (non-zero open decisions), unsupported claims (athletes with no durable
  observation, marks with no reproducible reduction) and an unverified export block completion.
* **The evidence is the digest.** The seal's sha256 is rendered from the counts, the retained
  findings (sorted, so gap order is not evidence), the open-work counts (all zero on a seal) and the
  workbook check — its row and sheet counts and the sha256 of the export it read, so two exports with
  the same row count cannot share a seal. The observation date is deliberately *not* in the digest:
  the same census sealed on two days is the same census. Identical evidence renders identical bytes;
  one moved count, or one different workbook, renders a different seal.
* **Sealing is idempotent.** Sealing an already sealed census returns the same seal rather than
  minting a second digest for the same census.
* **The CLI derives the phase from artifacts, never from arguments.** `census-service seal` walks
  the ladder only as far as the store's own artifacts allow — durable observations, consolidated
  snapshots, review cases or verdicts, a coverage classification, an exported workbook. A store
  missing one of them cannot reach the export phase, so the phase a seal is granted in is a
  recorded fact.
* **The workbook is checked by reading it.** The seal opens the export, hashes its bytes (streamed,
  so a large workbook is never materialised twice), requires the `Athletes`, `Coverage` and
  `Run Metrics` sheets, reconciles the cohort that `Run Metrics` names against the store's count,
  and reconciles the `Coverage` sheet's jurisdiction rows against the classifier. A disagreement is
  a named discrepancy and a refusal, never a smaller number reported as success.
* **What the seal does not verify, it does not claim.** Cell-level reconciliation of the
  multi-million-row `Performances_00N` sheets belongs to the workbook verifier. The seal records
  `rows: 0` rather than a count it did not take.

## Consequences

* "Finished" means sealed: the terminal state carries the evidence and the digest, so a consumer of
  the census — a recruiter opening the workbook, an operator reading `out/seal.json`, a later
  verification pass — can distinguish a final census from a partial one without trusting a label.
* A refusal is actionable by construction: it names the acceptance item and the numbers
  (`Run Metrics cohort 307652 != store 307653`).
* The ladder is derived where the census is assembled — the store's artifacts — rather than
  duplicated in the workflow journal. The per-jurisdiction Restate objects keep recording their own
  coarse stages (teams, rosters, consolidate); they are sub-work of a census, not the census, and
  neither surface claims to know what the other has done.
* A seal costs one coverage classification, one census projection and one streamed hash of the
  export. That is deliberate: the point of the gate is that it reads the artifact it certifies.
