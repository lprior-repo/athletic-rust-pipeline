# Explicit installation enrollment and legacy execution fences

Date: 2026-10-14 UTC. Implementation integrated; runtime checks are in progress.

## Adopted capability and compatibility

This is enrollment-only support for the reusable production architecture in ADR-038.
The existing stopped Store must have schema 2, key format 1, fresh lifecycle 2, empty
logical evidence/receipt state, and absent or empty known artifact roots. Valid immutable
profile registration and existing StoreIdentity may preexist. Unknown or historical state
refuses; no historical population is adopted or migrated.

The constrained InstallationConfigV1 binds initial profile, bounded service resources,
source-registry reference, operating-defaults reference and EnrollmentOnlyV1 capability.
A distinct InstallationConfigId hashes canonical configuration. The installation uses the
existing retained StoreIdentity. Service/CLI resolve the actual sorted compiled source
catalog and defaults exactly before opening the enrollment control. The lower-level Store
API validates structural references and the existing profile compiler; it cannot certify
crawl/service semantics or qualified source execution.

A private, non-Clone InstallationControl owns the existing Fjall lock continuously. It has
no generic Store escape. Enrollment prechecks eligibility, initializes ordinary durable
lineage if absent, rechecks under append serialization, then writes configuration,
enrollment and lifecycle 3 in one SyncData batch. Identity creation is a separate existing
durable operation and can survive a later enrollment failure. Exact repeated enrollment
returns Repeated; changed configuration is an immutable conflict.

The supported lifecycle ceiling is 3; new fresh qualification Stores still use lifecycle 2.
Generic Store open/open_existing/open_with and serve refuse enrollment-only execution before
recovery or listener/browser/source work. Diagnostic inspection and validated backup/restore
remain available. Coherent enrollment cannot enter legacy migration, even after a schema/key
marker downgrade. Non-enrolled schema-1 raw backup/restore compatibility remains unchanged.
Installation-bearing candidates must pass bounded semantic validation before restore promotion.
Whitespace lifecycle markers use the existing trim-before-integer interpretation.

The existing RunManifest, CensusRun, workflow keys, entity identities, source observations,
receipts, seals and historical publication artifacts are not rewritten. Enrollment does not
implement acquisition, synchronization, retained-evidence reuse or selected publication.

## Real control surfaces

- census-service installation defaults
- census-service --store DIR installation enroll --file CONFIG.json
- census-service --store DIR installation show

The shared existing bounded regular-file/version reader is reused. Defaults opens no Store.
JSON outcomes distinguish enrolled, repeated, shown, not_enrolled, defaults and failed.
Unknown capability is a serde input refusal. Domain errors retain UnsupportedVersion,
InvalidServiceResource, Profile and Canonical alternatives; no fictitious domain variants
are advertised.

## Scope of preservation evidence

Known artifact roots http, out, entities, journal, capture and captures must be absent or real
empty directories. A symlink, non-directory or any directory entry refuses. Checks neither
recurse nor delete. Arbitrary external operator-selected destinations are not inventoried.
Fjall may update physical lock/recovery/maintenance files when opened. Refusal witnesses cover
project logical rows, counters/generations and retained known artifacts, not a byte-identical
whole filesystem. Ordinary external filesystem changes are not atomic with the Fjall batch.

## Review and observed evidence

The detached 45-file proposal was independently reviewed and pinned at patch SHA256
0366bfa4e72f659e1e92d976dbc5f985d74ec721f1e6ec92463f4a517590a1b4.
Review corrected early enrolled migration refusal, physical artifact emptiness, historical
schema-1 backup compatibility, whitespace lifecycle classification and a restore regression
that had tested a nonempty destination rather than reaching candidate validation.

The first integrated format/build attempt exposed test module path resolution errors.
Explicit paths now bind the installation fixture/lifecycle/preservation modules under their
actual tests directory. The first build did not execute tests and is not passing evidence.
Executed results are recorded below; the failed attempt remains part of the evidence history.

## Specification acceptance mapping

- Spec sections 2, 4, 9, 12 and 14: explicit bounded installation configuration, preserved Store
  lineage, durable enrollment/repeat/conflict and migration/backup safety. Enrollment-only
  support is narrower than a reusable production execution engine.
- Sections 3, 6, 10 and 11 remain open for profile-bound source discovery, distinct physical
  capture attempts, qualified interpretation/reuse, resumable synchronization and reparse.
- Sections 5, 7, 8 and 13 still require full identity/affiliation/comparison integration and
  selected-profile workbook plus normalized-snapshot parity. Separate result-status policy-18
  evidence records the current comparable-mark repair.
- Sections 16 and 18/release obligations remain open: actual predecessor executable refusal,
  native precommit/postdurable fault/replay evidence, hostile-input/property/fuzz campaigns,
  full release/security/resource/performance qualification and end-to-end workbook readback.

No live census or production Store was used. Full specification delivery is not verified.

## Executed core results

The first completed core run, nextest `0bc4bcdf-c680-41fe-9073-263dbe710524`, ran
304 tests: 302 passed, two failed, 1441 skipped, 21.618 test seconds. One old future-lifecycle
fixture still treated 3 as newer than the supported ceiling 2. It now independently tests
future 4/u32::MAX against ceiling 3; lifecycle-3 orphan refusal remains explicitly covered
by the new installation tests. No production refusal was weakened.

The other failure was `backup_tests::backup_refuses_a_socket_in_the_store_tree`, whose
Unix socket setup returned EPERM in this sandbox. An escalated isolated attempt failed
before command execution in the sandbox launcher (exit 101). That test remains unverified
here and must run on the home environment; it was not deleted or marked ignored.

After the fixture correction, nextest `feaa35f3-2d38-41d3-bdba-178167723c8c` passed
303/303 selected tests, with 1442 skipped, 16.434 test seconds and Moon exit 0. The filter
explicitly excludes only the blocked socket case from the original core selection. This
includes real Store enrollment/reopen/relocation, atomic commit sequence/repeat witnesses,
historical state refusal, migration/format/profile/backup regressions, policy-18 preservation
and result-status/authority/report-selection regressions. It does not substitute for native
crash injection or the actual predecessor executable.

The acquisition CLI test now exists as an independently reviewed child module: it invokes
real fetch --refresh against an inert loopback sentinel, checks exact typed refusal text,
zero observed connections, full bounded logical rows and retained artifacts. The subsequent service/CLI lane executed this regression successfully, as recorded below.

## Final focused CLI and descriptor evidence

- Actual service/CLI lane: nextest `4a7aaa2a-c726-476f-b850-2f664260bf62`,
  43/43 passed, 883 skipped, 0.314 test seconds, Moon exit 0. Selected service library,
  census-service binary, installation_cli and profile_cli targets. The new fetch --refresh
  refusal/sentinel test and actual serve-before-bind refusal both passed.
- Complete profile_execution binary: `f0e7eabb-4979-4f03-926c-acc1a49f2041`,
  17/17 passed, zero skipped, 0.138 test seconds, Moon exit 0.
- Named installation coverage reconciliation found all 45 expected new test names in the
  core and service/CLI passing logs, with zero missing names. This is selection coverage,
  not a source coverage percentage or crash/proof certification.
- A final basic run passed formatting and all-target compilation, then source lint failed
  on manual modulo divisibility in installation/resources.rs. It was changed to the
  equivalent is_multiple_of predicate; final rerun results follow in the consolidated record.

## Home verification commands

Run from the repository root with the pinned toolchain and isolated test resources:

```sh
env -u CI tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
env -u CI tools/moon-local run pipeline:tests -- --build-jobs 2
env -u CI tools/moon-local run pipeline:gate -- --release
```

The complete tests command includes the unchanged Unix-socket backup case excluded locally.
The release command is the repository full release entrypoint, not a claim that all native
prerequisites are installed or that the seventeen fault obligations have passed. Review its
reported obligations and retain native fault, predecessor-reader, fuzz, security, resource,
performance and workbook-readback evidence. Use only isolated qualification stores; these
commands do not authorize opening production state or starting a live census.

## Final basic checks

After the equivalent divisibility-predicate correction, Moon completed all three tasks:
format check, workspace/all-target/all-feature compile check, and strict production source
Clippy (warnings denied). Overall exit 0, 2m19.390s. The initial lint failure remains recorded;
the repair did not relax a lint or alter the accepted numeric range. Focused resource-domain
and reader callback regressions are recorded in the consolidated evidence ledger.
