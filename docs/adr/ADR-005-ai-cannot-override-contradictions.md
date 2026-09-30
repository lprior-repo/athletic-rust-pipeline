# ADR-005 — AI cannot override deterministic contradictions

Status: accepted policy; acceptance evidence is tracked separately from implementation claims.

## Context

A plausible wrong merge contaminates every downstream record. Retaining an unresolved case is
safer than allowing model confidence to substitute for evidence. Local GPU review is scarce and
should not be spent on cases deterministic rules can resolve.

## Decision

Normalize evidence, retrieve candidates and check contradictions deterministically first. Only
residual ambiguity receives independent advice from both approved local Qwen lanes.

The semantic answers are `SamePerson`, `DifferentPerson` and `InsufficientEvidence`, bound to the
prepared evidence and policy/model revisions. Models receive bounded structured source-linked facts
and retained contradictions, not authority to invent facts or browse private admissions data.

Rust alone accepts identity decisions. Acceptance requires sufficient admissible corroboration,
satisfaction of deterministic constraints and the required reviewer agreement. Agreement cannot
override a hard contradiction. Malformed output, failure, disagreement or insufficient evidence
remains `REVIEW`; there is no cloud fallback.

## Consequences

Cache valid advice only under its exact evidence/policy/model binding; material evidence changes
invalidate affected advice and decisions. Retain responses and reversible decision provenance.
Models never calculate marks, establish graduation facts or fabricate affiliations/contact details.
Candidate generation and score thresholds do not themselves constitute acceptance.

[Architecture §§8–9](../../ARCHITECTURE.md#8-identity-review-32-34) owns the semantic contracts;
[the delivery plan](../NATIONAL-CENSUS-PLAN.md) owns dual-lane integration and adversarial acceptance.
