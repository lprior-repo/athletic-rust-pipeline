# Native Mac Moon launcher

`tools/moon-local` resolves and fingerprints native Apple compiler, archiver and linker
executables rather than requiring a Linux bundled linker or Apple `ar --version`.
Linux tool selection/fingerprint meaning remains unchanged. Physical root resolution
and SHA-256 fallback work without GNU coreutils. `tools/moon-cargo` preserves set/unset
job-override semantics on Bash 3.2. No alternate build entrypoint is introduced.

- PASSED: eleven launcher and six Cargo-wrapper regressions, 17 passed, 0 failed,
  0 skipped; Nextest `75e6cf3f-1ee2-44ca-acc3-87698d8cd419`, 4.165 s test time,
  4m52.851s total Moon task.
- PASSED: final Moon formatting (11.172s) and workspace all-target/all-feature check
  (55.020s), shell syntax and Git whitespace checks.
- PASSED: isolated shell fixtures for both Apple hosts, system-shim resolution,
  original/patched Linux fingerprint byte equality, unchanged fingerprint preservation,
  missing linker, hash failure and failing probes that emit plausible paths.
- Static review found two masked probe-exit failures; both were corrected and covered
  by executed regressions before acceptance of this patch.
- PENDING: actual native Mac launcher and Bash 3.2 smoke, native format/check results.
  Fixtures on Linux do not establish macOS execution.
- NOT RUN: Clippy, full release/fault/fuzz/resource qualification. Linux-targeted
  build/bench/fetch lanes remain Linux-specific; this is not cross-compilation support.

The changed wrapper/test surface was independently reviewed for failure propagation,
selected-tool identity, cache invalidation and shell compatibility. No census, service,
production data, credentials or security settings were changed by this patch.

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:fmt
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check -- --jobs 1
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p xtask --test moon_local --test moon_cargo --no-capture --no-tests fail
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:lint-src
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:gate -- --release
```
