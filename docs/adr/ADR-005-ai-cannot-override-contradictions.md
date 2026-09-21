# ADR-005: AI cannot override deterministic contradictions

## Status

Accepted as policy. Partial implementation: review plumbing and case identity exist in the
domain/runtime; the GPU lanes (RTX 5090 + RTX 3090 llama.cpp) are the intended consumers.

## Context

Identity reconciliation is the place where a plausible language model answer is most dangerous.
Given two candidate records, a model can always produce a confident "same person", including
when the structured evidence says the two cannot be one athlete (different birth-year evidence,
overlapping meets in different states on the same day, contradictory school enrollment,
impossible progressions). Letting model output outrank those constraints would convert the
census's identity layer into a source of silent, unauditable errors — the exact failure the
correctness objective forbids.

At the same time, deterministic rules cannot resolve genuinely ambiguous cases (common names,
school transfers, missing grade evidence). That residue is where GPU inference is worth
spending.

## Decision

1. **Deterministic first.** Normalization, hard-contradiction detection and scoring run before
   any model is consulted. A case the constraints can settle is never sent to AI.
2. **AI is bounded to one question.** The model returns exactly one of:

   ```rust
   enum ModelIdentityAssessment { SamePerson, DifferentPerson, InsufficientEvidence }
   ```

   over a compact structured evidence packet (source subject, candidate, shared meets, event
   overlap, explicit contradiction list) — not raw HTML.
3. **Rust adjudicates.** The model's output is an *input* to a deterministic adjudicator, never
   the final answer by itself. If hard constraints fail, **AI cannot create a match** — the
   contradiction stands regardless of the model's confidence.
4. **Low confidence becomes REVIEW,** not a guess. `InsufficientEvidence` and low-confidence
   assessments produce a retained review case, visible in the workbook's review sheet, rather
   than a silent merge or a silent split.
5. **Review cases have stable identity.** `review:{evidence_digest}:{policy_revision}` — the same
   evidence package reuses a prior review instead of re-burning GPU time, and a policy change
   invalidates it explicitly.
6. **AI never decides marks.** PR comparison is event-aware deterministic ordering (§23); no
   model output may reorder performances or choose a "better" mark.

## Consequences

* The identity layer is reproducible: same evidence + same policy revision = same decision,
  independent of model temperature or availability.
* Model downtime degrades the census to "more review cases retained", never to wrong merges —
  which is why the pipeline can run for days unattended.
* Every AI-touched decision is auditable: the evidence packet, the model verdict, the confidence
  and the deterministic adjudication are all retained.
* GPU inference is spent only on the residual ambiguity, keeping both 5090 and 3090 lanes
  productive on genuinely hard cases.
