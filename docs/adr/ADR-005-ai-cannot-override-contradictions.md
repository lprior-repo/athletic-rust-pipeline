# ADR-005: AI cannot override deterministic contradictions

## Status

Accepted as policy; partially implemented — the deterministic adjudication and the local reviewer lane
exist and are wired together. The implemented answer is `AssistantVerdict::{Select, Unresolved}`
(`src/runtime/reviewer/model/verdict.rs:5-15`), **not** the objective's `ModelIdentityAssessment`.

## Context

Identity reconciliation is where a plausible language-model answer is most dangerous. Given two candidate
records a model can always produce a confident "same person", even when structured evidence says the
two cannot be one athlete (contradictory birth years, meets in two states on one day, contradictory
enrollment, impossible progressions); letting that answer outrank the constraints would make the
identity layer a source of silent, unauditable errors. `ARCHITECTURE.md:87-88`: "AI review is a scarce
adjudication resource for identity conflicts only, and cannot override a deterministic contradiction."
Rules cannot settle genuinely ambiguous cases — common names, transfers, missing grade evidence — and
that residue is where GPU inference is worth spending, subject to `SCOPE.md:21`: "model output requires
source review and executable verification".

## Decision

1. **Deterministic first.** Normalization, contradiction detection and scoring run before any model is
   consulted; a case the constraints can settle never reaches AI.
2. **AI is bounded to one question** over a compact structured packet, never raw HTML (`ReviewInput`,
   `src/runtime/protocol.rs:154-158`), returning a candidate selection or `Unresolved`.
3. **Rust adjudicates.** The model's output is an *input* to `decision::apply_review`, never the final
   answer; when a hard constraint fails the selection is rejected and the row falls back to review.
4. **Poor evidence becomes REVIEW, not a guess.** `Unresolved`, a failed review call, or a rejected
   selection all produce `RowResolution::ReviewRequired`.
5. **Review cases have stable identity** keyed by content plus protocol revision, so a case reuses a
   prior assignment instead of re-burning GPU time, and a policy change invalidates it.
6. **AI never decides marks.** PR comparison stays event-aware and deterministic (`ARCHITECTURE.md:86-87`).

## Consequences

* Reproducible: same evidence + same policy revision = same decision, regardless of model temperature or availability.
* Model downtime degrades the census to "more review cases retained", never to wrong merges — which is
  what lets runs continue unattended.
* Every AI-touched decision is labelled `AcceptanceMethod::{Deterministic, LocalReview}`
  (`src/runtime/row_protocol.rs:98-102`), counted separately (`src/runtime/run_protocol.rs:89-101`) and
  checked by two independent verifiers (`.../checks/positive/mod.rs:160-177`, `bundle_verify/binding.rs:146-150`).
* GPU inference is spent only on the residual ambiguity that reaches the reviewer lanes.

## Alternatives considered

* **Pure model adjudication.** Rejected: a hallucinated merge would be unauditable, unreproducible, and defeats the verifiers above.
* **A numeric confidence threshold as the review trigger.** Not implemented — see the gap below; review
  is triggered by rule outcomes, which keeps the trigger explainable.
* **Deterministic-only, no model.** Rejected: the residual ambiguity is real, and `Unresolved` is an
  explicit acceptable answer.
* **Let AI rank or choose "better" marks.** Refused outright (`ARCHITECTURE.md:86-87`).

## Evidence — what exists, what is missing

Implemented: `SearchCompleteness`/`Decision` and `hard_eligible` (`src/domain/decision.rs:27-39,41-50`);
a decision function requiring a complete search and one hard-eligible candidate
(`src/domain/decision/pipeline.rs:92-124`); verbatim refusals pinned by tests — "selection cannot
override incomplete discovery" (`src/domain/decision/review.rs:19-20`) and "selection cannot override
insufficient or contradictory evidence" (`:22-27`), plus non-eligible-candidate and
non-distinguishing-evidence refusals (`:28-38`, `tests.rs:158-168,222-228,269,323-335`); wiring in
`applied_resolution`, where the verdict flows through `ReviewChoice` into `decision::apply_review` and
rejection yields `RowResolution::ReviewRequired` (`src/runtime/row_worker/review.rs:152-191`); a bounded
model contract whose prompt says missing fields are unknown and to answer unresolved when evidence
cannot distinguish (`src/runtime/reviewer/model/validation.rs:6-8,14-19`, `model/prompt.rs:12-17`);
case identity via `case_key` (`src/runtime/review_case.rs:23-37`, lanes `:12,169-172,279-291`), a
blocked lane answering with its durable failure (`RESTATE_WORKFLOWS.md:274,439`), and a model 500
classified `FailureCode::RetryExhausted` (`src/runtime/reviewer/publication.rs:211-214`).

Missing or different from the objective:

* No `ModelIdentityAssessment`: the implemented question is "choose among supplied candidates or answer
  unresolved" (`verdict.rs:5-15`), not "is this pair one person" — the implemented contract governs.
* The literal key `review:{evidence_digest}:{policy_revision}` does not exist; the implemented key is a
  content fingerprint including protocol revision (`src/runtime/review_case.rs:32-37`); intent matches, format not.
* "Poor confidence yields REVIEW" has no code-level confidence rule: `ConfidenceScore` is unused by the
  reviewer path (`src/domain/facts/scalars.rs:44-48`), and `candidate.evidence_strength` gates nothing
  (`src/domain/decision.rs:50`).

No live inference call was made here; lane and validation claims come from code and tests.
