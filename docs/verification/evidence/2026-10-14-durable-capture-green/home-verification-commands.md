# Full home-workstation verification commands

These are proposed commands, not execution evidence from this worker. Run from the repository root in the intended home checkout; no absolute home path or historical `HOME` value is assumed. Verify the intended commit and clean source before running, record exact commands/statuses and retain all logs. Do not substitute the recovery toolchain paths for the home machine's configured toolchain.

Repository reference: `AGENTS.md`, `xtask/README.md` and `moon.yml`. Prerequisites include Moon 2.2.4, pinned `nightly-2026-04-27`, the configured target/components, Cargo-selected Nextest, Bash, jq, coreutils and native build tools. Every build/test/check/lint/format/fault command must enter via `tools/moon-local`. `env -u CI` matters even when CI has the value `false`.

## Final source and full workspace execution

Run sequentially and retain each terminal result:

```sh
env -u CI tools/moon-local run pipeline:fmt
env -u CI tools/moon-local run pipeline:check
env -u CI tools/moon-local run pipeline:lint-src
env -u CI tools/moon-local run pipeline:tests
tools/moon-local ci --force --summary detailed
```

The check target compiles all workspace targets/features; the source lint target covers lib/bins/examples and deliberately preserves the forced arity review warnings. Tests and CI are not the release gate or full product qualification. Forced CI avoids accepting a zero-affected-target run. It can reuse compiler state; it is not a cold-compiler benchmark.

## Portable release and full acceptance wrapper

```sh
env -u CI tools/moon-local run pipeline:build-portable
env -u CI tools/moon-local run pipeline:native-resources
env -u CI tools/moon-local run pipeline:gate -- --release
```

Add `--force` to `pipeline:build-portable` when the portable build must execute rather than hydrate an artifact. Record the actual portable binary hashes. Do not replace the release gate with a build, a focused test, `report-test`, or iteration CI.

The current release wrapper runs its durability lane. It wires the native-fault feature endpoint, portable endpoint, receipt oracle and VM host; it requires verified Restate, VM/capture resources, isolated ports and owned scratch. `tools/durability/README.md` is the detailed resource/run reference. The default Restate lookup is the documented pinned 1.7.10 install; set `RESTATE_SERVER_BIN` only to a separately verified path when needed. The complete native lane is not established until all required scenario/subcase receipts exist. Do not reboot the shared host, alter its clock, fill production storage or open a live Fjall root from another process.

For a separately assigned release component, the supported complete commands are:

```sh
env -u CI tools/moon-local run pipeline:gate -- --release --lane contract
env -u CI tools/moon-local run pipeline:gate -- --release --lane panic-extraction
env -u CI tools/moon-local run pipeline:gate -- --release --lane scan
env -u CI tools/moon-local run pipeline:gate -- --release --lane ratchet
env -u CI tools/moon-local run pipeline:gate -- --release --lane durability
env -u CI tools/moon-local run pipeline:gate -- --release --lane perf
```

A single selected lane cannot replace the umbrella release result. No baseline update, quality waiver, license/cargo-vet reintroduction or retired mutation lane is authorized by this handoff.

## Measurement, parser fuzz and named proofs

The existing three measurement targets, if assigned on a quiet machine, are:

```sh
env -u CI tools/moon-local run pipeline:bench -- --bench core -- --noplot
env -u CI tools/moon-local run pipeline:bench -- --bench pipeline -- --noplot
env -u CI tools/moon-local run pipeline:bench -- --bench snapshot -- --noplot
```

Retain workload/corpus/binary identity, units, warmup, samples, environment and profiler context. These benchmarks do not by themselves measure the new capture-admission boxing path or prove whole-census memory/disk/throughput behavior.

The reference exposes no proof/Kani/Verus command in xtask or the release wrapper; do not invent a Moon proof lane or infer named proof-kernel coverage from release success. Likewise, do not claim a parser-fuzz campaign ran merely because tests or a release gate passed. Native faults, parser fuzz, named proof kernels, security/dependency gates and whole-workload resources remain separately evidenced obligations. The current command references do not supply a complete Moon fuzz-campaign invocation, so no bare Cargo fuzz command is substituted here.
