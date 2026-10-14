# Contextual documentation correction

TASK: correct the stale native-fault execution description in Architecture §11 only.

OWNERSHIP: detached `ARCHITECTURE.md` proposal and contextual `changes.patch`; Root reviews and applies. No live repository write occurred.

INPUT CONTRACT: base hash is in `base-pins.sha256`, captured from implementation commit `3708d6bab0b08c390887c562a89e8c6714255a5d`. `tools/gate.sh:259–306` implements a release-only durability lane with owned scratch, feature endpoint, plain endpoint, receipt oracle, VM host and capture wiring; `tools/gate.sh:516` invokes it. `tools/durability/README.md` independently documents this invocation. Those references are hash-pinned in the parent proposal's `input-references.json`.

OUTPUT CONTRACT: preserve section numbering and all existing architecture semantics. Replace the inaccurate claim that the release gate does not execute native faults and remove the obsolete suggestion to add the already-present native lane. Preserve the separate parser-fuzz requirement and the distinction between actual scenario results and prerequisites/skips. No native fault run or qualification is claimed by this text repair.

ACCEPTANCE: inspect the contextual patch against those two sources; verify the exact base hash before application. Text-only patch inspection is not a Rust gate. Do not apply it over a different base without review.

HANDOFF: `changes.patch`, full proposed `ARCHITECTURE.md`, and base/proposal SHA-256 manifests. No source, frozen evidence, release policy, baseline, obligation or historical result is changed by this worker.
