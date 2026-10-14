# Typed owned-parser resource failures

This prerequisite separates malformed source documents from local resource refusal at the
actual public owned-meet parser and its existing fetch, capture, collector and Restate callers.
It does not introduce the full interpretation descriptor or qualify reusable production.

## Adopted behavior

`parse_owned_meet` now returns exhaustive, nonserialized Malformed/Resource alternatives.
Resource failures preserve typed limit/request, arithmetic operation, allocator source and
original document/byte locator. Only genuinely malformed documents produce the historical
serialized Malformed verdict. Existing serialized v3/v4 shapes and phase constants remain.

Admission agrees with the pinned decoder's actual maximum of 127 nested containers. Existing
small envelope diagnostics preserve exact compact bytes through the prospective 1024-byte
rendered diagnostic budget. Larger diagnostics become typed resource refusal, preserving the
exact required byte count; no shortened string is written under an old immutable receipt key.

Actual acquired raw bytes are archived before the metadata carrier releases its body. Original
capture digest/path and explicit retained/report-only receipt status survive parser or recording
pressure. A recording Resource is carried with its real fields, without inventing a parser
failure. Original byte offsets remain visible in unfinished locators and resource receipt keys.
The existing collector stops later supplied intake and preserves owed URLs rather than claiming
malformed, empty success or complete projection.

New resource receipts use the existing resource family separately from historical malformed
acquisition receipts. Old v3/v4 resource-misclassified and large-malformed payloads remain exact
through replay/reopen. No historical record is rewritten or reinterpreted as qualified lineage.

## Review and execution boundaries

The independently reviewed detached proposal comprised 39 files with 26 exact preexisting base
pins. Root applied it contextually, retaining current result-status registrations, and formatted
through Moon. All-target/all-feature compile check passed, exit 0 (39.817 seconds total).
Focused parser/collector/service regressions and strict source lint are pending at this entry.

The capture replay test deliberately supplies from_cache=false metadata to the real retention
helper, then exercises the real cached owning read and full retained payload equality. This is
retention-branch/replay evidence; it is not an executed physical-network first-acquisition test.
Only explicit fallible reservation boundaries are classified as allocation failures. This work
makes no allocator-wide graceful-OOM claim for serde Value, formatting, Box or other infallible
allocations. Full parser fuzz/property/mutation and resource measurement remain unqualified.

## Executed focused lane and subsequent finding

Nextest `5fa55cb2-9bb4-4b2c-9c6a-e0e0e916a2b9` passed 86/86 selected crawl/service
library tests, 2026 skipped, 27.197 test seconds, Moon exit 0. This includes original typed
boundary, historical receipt, collector recording-pressure, raw retention, cached replay and
terminal service-error regressions. It does not certify unselected service parity fixtures.

A subsequent integration review found an uncovered relocation defect: absolute archive paths
were persisted under a now-stable resource identity, while collector error displays also
included local paths/retention status in hashed payloads. Reopening at another root could
therefore conflict or create another resource identity. This finding blocks final slice
acceptance despite the passing 86 tests. The adopted repair separates stable original cause
and archive-relative persisted locators from current operator path/display, under an explicit
new resource identity version preserving old records. Its execution evidence must be added
before claiming the defect is resolved.

The integrated parser plus geocode transport changes subsequently passed all three basic Moon
tasks: strict production source lint, formatting check and all-target/all-feature compilation,
overall exit 0 (1m41.595s). This confirms compilation/style of that snapshot, not resolution
of the separately identified relocation invariant. No lint allowance or test skip was added.

## Relocation repair executed

The genuine physical-move regression failed before repair with an immutable owned
receipt conflict. The repair versions typed receipts as partial/resource-v2, keeps
archive-relative persisted locators and stable original causes, and retains current
physical paths only in runtime/operator reporting. The repeated 90-test focused lane
passed, followed by all three basic checks and actual two-test service parity.
The consolidated [repair evidence](2026-10-14-parser-repair.md) and exact source
ledger retain every failed/interrupted attempt and remaining qualification gap.
