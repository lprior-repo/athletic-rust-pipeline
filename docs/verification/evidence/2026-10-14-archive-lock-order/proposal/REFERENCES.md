# Sources read for the narrow repair

Repository:
- AGENTS.md
- ARCHITECTURE.md, especially §§4,5,7,10,11
- docs/adr/ADR-001-fjall-primary-store.md
- docs/adr/ADR-002-restate-owns-retries.md
- docs/adr/ADR-012-single-acquisition-plane.md
- docs/adr/ADR-037-blocking-lane-admission.md
- Current Crawl cache archive/admission/publication/inventory/reservations/eligible/I/O/capture/envelope and acquisition read/retention/session/dispatcher/tests
- Current Store acquisition terminal/read/backing/capability and receipt scan surfaces
- prepared-recovery-minimal-review/M1-PREREQUISITE.md, copied exactly as ADOPTED-PREREQUISITE.md

Recovered skill sources:
- skills/rust-contract/SKILL.md
- skills/rust-contract/references/type-contract-checklist.md
- skills/holzman-rust/SKILL.md
- skills/holzman-rust/references/nasa-jpl-standards.md
- skills/holzman-rust/references/zero-cost-abstractions.md
- skills/scott-ddd-refactor/SKILL.md
- skills/scott-ddd-refactor/reference.md
- skills/scott-ddd-refactor/checklist.md
- skills/scott-ddd-refactor/decision-matrix.md
- skills/async-rust-reviewer/SKILL.md
- skills/async-rust-reviewer/references/structural-correctness.md
- skills/async-rust-reviewer/references/cancellation-safety.md
- skills/async-rust-reviewer/references/spawn-discipline.md
- skills/async-rust-reviewer/references/send-sync-ownership.md

This work is an internal lock-lifetime correctness repair. It does not redesign inventory traversal or make allocation/latency/throughput claims. Required execution gates remain Root-owned and not run by this worker.
