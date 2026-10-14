# Dated affiliation candidates and bounded capture evidence

Date: 2026-10-14 UTC. Baseline: PR #27 merge
`ca247d173ef0f36765a1d021ac681575abe861e1`. Code commit: `89e9fa67de617e736c673ee18578254b16263f82`. Integration evidence is still being collected.
Exact path/blob coverage is in the adjacent `2026-10-14-candidate-and-capture-files.json`.
No live census, production Store or production service is used by this change.

## Contracts and compatibility

The new pure domain module supplies locally checked, dated affiliation candidates. It preserves
source person/owner, acquisition occurrence, interpretation, effective interval, known revision,
withdrawal/departure and unresolved co-op campus distinctions. Its raw references and claimed
bindings are not accepted source, identity, current-school or publication authority. It changes
no historical `AthleteAffiliation` shape and does not activate selected-profile publication.

Selection admits conservative pair work and nonshared material comparison bytes before sorting,
replay equality or evidence integrity comparisons. Shared immutable slices require matching
pointer and length before the byte scan can be skipped. Exact content comparison remains mandatory.
The candidate boundary limits legacy capture-date text to 64 bytes before parsing; existing
SourceCapture constructors/serialization and historical data remain unchanged. These are bounded
project-work contracts, not measured latency, allocator or whole-process-RSS guarantees.

Store raw observation scans and existing merged scans share one bounded physical decode loop.
The raw visitor retains every physical version and duplicate, never merges or publishes it, and
returns terminal counts only after EOF. Callback values remain provisional on later refusal.
Journal membership inventories similarly bind complete physical namespace scans to the supplied
Store view. Logical root enumeration reuses legacy/alias decoders and validates ordinal closure;
recognizing a Store-v2 alias does not prove the backing object closure.

The real frozen capture consumer shares one 200-GiB physical key-plus-raw-value budget across
nonathlete merged/observed scans, with the existing 20-million-row table cap and 1-MiB record cap.
The repeated coach scan is charged again. This is stricter future capture admission: physical
historical versions and key overhead can refuse recapture even if merged output would fit.
Athlete preprocessing, lineage/fingerprint scans and backend materialization are separate,
unqualified resource surfaces. A successful stream's schema/token/digest meaning is unchanged;
policy remains 17, frozen codec/token 3 and physical Store schema 2. Historical reopen is unchanged.
Resource/writer refusal cannot finish a partial stream or publication and preserves predecessors.

ADR-038 reconciles the full v2 reusable-production target with isolated fresh qualification and
historical 49-jurisdiction census claims. It does not implement enrollment, remint IDs, migrate a
store, authorize production operation or certify annual reuse. The supplied canonical specification
remains byte-for-byte SHA256 `a3b687cc06952dbceaeda2f21300e4769acb96130570fc1174ff70210a09f11e`.

## Review and observed repair evidence

Independent candidate review found unmetered material equality, unbounded legacy date text and
positive-path/byte-charge test gaps. Two new regressions failed before the production correction
and one shared-buffer regression passed; all 33 candidate tests passed after correction
(run `288d173d-5161-452f-b5a6-dbe3b05d40ce`). Strengthened tests then exposed an incorrect test
expectation: equal relationship candidates retain the first stable candidate while keeping both
audit facts. The test was corrected to that existing contract, not production behavior.

The later run `cbc304b1-60ba-4884-9342-42cc7da62629` passed 38 candidate tests and two capture-admission
unit tests. Run `12a78e29-f94f-49c3-bb49-44ce73be25a2` passed all seven additional tests: three
material-work branches, independent physical-byte/duplicate/raw-publication tests, and two actual
capture shared-budget/repeated-coach tests. The first raw-byte test compile failed because its
fallible assertion macro returned a test error inside a StoreResult callback; assertions moved
outside the callback without changing production or weakening the expected values.

Earlier combined frozen/domain/Store run `2186e7f9-3c63-46ab-a405-abd4f4fedd4e` observed 229 passes,
zero failures and one unfinished test. The remaining resource-preservation resume case stalled
past 180 test seconds and the run was interrupted normally. An isolated rerun with temporary
phase markers passed that case in 3.521 seconds (`97ebed7d-bddc-4a01-9eab-c2e2b2794a44`). Markers
were removed. No cause or deadlock repair is established, and the combined run is not a pass.

Membership's 48 focused tests passed in that combined run, including seven independently fixed
fingerprint vectors, legacy/alias closure, exact ordinal/key/descriptor refusal and provisional
callback handling. Review still discloses backend pre-materialization and decoded JSON allocation
as unresolved selected-runtime resource qualifications.

## Specification acceptance mapping

This mapping narrows the contribution of this change; it does not close complete sections.

| v2 requirement | Contribution | Remaining acceptance |
| --- | --- | --- |
| 2, 14, 17: ownership and reusable annual state | ADR-038 explicit lifecycle and historical compatibility | Versioned enrollment, multi-cohort runtime and isolated migration qualification |
| 5: stable and reversible identities | Candidate references preserve supplied IDs | Stable forward allocation and reversible accepted binding producer/consumer |
| 6: capture/interpretation identity | Candidate occurrence/material distinction; bounded namespace inventories | Source-neutral retained closure, exact parser/config/member versions and required-family activation |
| 8: dated affiliations | Pure candidate temporal/owner/person/co-op/withdrawal rules | Qualified official roster/accepted identity and participation bridge, current contact/affiliation publication |
| 9, 12: bounded durable reads and recovery | One-view raw/member reads, exact physical admission, unchanged history | Backing-object/decoded resource qualification; unresolved combined recovery-test stall |
| 13: consistent workbook and normalized snapshot | Existing frozen capture now uses shared admission and preserves writer errors | Checked selected view, eight normalized families, independent readback and atomic shared generation |
| 16: verification | Scoped tests and changed-surface reviews with exact results | Current final basic gates, whole release, all 17 native faults, bounded fuzz, independent oracle and representative resource/performance evidence |
| 1, 3, 4, 7, 10, 11, 15, 19 | No completed acceptance claimed by this batch | Continue existing full-spec implementation and first five-state/UP qualification |

## Home verification

Run from the repository root through the configured pinned toolchain:

```sh
tools/moon-local run pipeline:fmt
tools/moon-local run pipeline:check
tools/moon-local run pipeline:lint-src
env -u CI tools/moon-local run pipeline:tests-scoped -- -p census-domain -p census-store -p census-report -E 'test(affiliation_candidates) | test(read::view::bounded) | test(read::view::raw_physical_tests) | test(read::view::membership) | test(export::dataset::frozen)'
tools/moon-local run pipeline:tests
env -u CI tools/moon-local run pipeline:gate -- --release
```

Release-wrapper success does not execute or replace the separately catalogued native fault,
parser-fuzz and representative-corpus resource/performance qualifications. Their exact supported
Moon commands remain in `xtask/README.md`, `docs/NATIONAL-CENSUS-FAULTS.md` and the existing plan.
No unexecuted lane is marked passed. Current Beads export is unavailable (`bd` absent); historical
interaction records are not treated as the current backlog.
