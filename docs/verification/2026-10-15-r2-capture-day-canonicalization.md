# R2 capture-day affiliation canonicalization

## Scope and status

This is a narrow runtime ownership refactor, not a delivery of time-scoped affiliation tenure.
The production report bridge currently constructs exactly four outcomes: `Resolved`, `Unknown`,
`Conflicted`, and `CoopCampusUnresolved`. Their representation and reducer now live in
`census_domain::affiliation_candidates::capture_day`; the report's
`CurrentAffiliationResolution` is a thin alias. The superseded report-local `combine` function
is removed.

Tests described here are added source, not observed passing results. Main owns integration and
execution. This isolated work intentionally ran no formatter, build, test, or release gate and
makes no performance or release-readiness claim. `bd prime` was attempted but `bd` is unavailable.

## Domain and boundary contract

The domain reducer receives an admitted canonical school and the retained row/capture references.
It does not establish source authority, school ownership, identity acceptance, archive integrity,
sport membership, or temporal scope. Those checks remain in the report bridge. In particular,
the bridge requires the original acquisition's UTC capture day to equal the selected profile's
as-of date and requires positive membership in at least one selected sport before calling the
reducer. A different capture day is not continuing tenure.

`observe_school` classifies an admitted cooperative program as campus-unresolved and an admitted
ordinary school as resolved. The reducer is per canonical athlete across selected sports; it
does not pick a favorite sport or silently discard another selected sport's conflicting school.
`withhold` applies the report's already-established missing-required-admission decision without
erasing a known contradiction. The report still decides which gaps require withholding:
unaccepted identity, unavailable same-day archive, or unresolved same-day school.

The domain transition contract is:

- Unknown plus an admitted school adopts that school's resolved or campus-unresolved outcome.
- Repeated resolved evidence for the same school unions references, sorts them lexically, and
  removes duplicates.
- Two campus-unresolved cooperative programs remain campus-unresolved, including distinct
  program IDs. This preserves the existing rule; it is not campus selection.
- Every other combination of admitted outcomes is conflicted. Later rows cannot clear conflict.
- Required-admission withholding turns every nonconflicted outcome into Unknown and preserves
  Conflicted.
- Unresolved outcomes expose no resolution references. The report's independent audit retains
  admitted source references and unavailable reasons, including outside-selected-sport evidence.

No `RawAssertion`, source activation, membership tenure, effective interval, relationship end,
non-school affiliation, or full `assess_current_affiliation` authority is fabricated. The separate
generic affiliation candidate and current-assessment types, outcomes, rules, and explicit future
proof-contract tests remain unchanged.

## Runtime-only state narrowing and persistence inspection

Before editing, repository-wide source searches found report `EndedWithoutReplacement` and
`NonSchool` only in the runtime enum/status method and three synthetic test users: geography
exclusion tests, selected-best writer refusal tests, and the affiliation-audit fixture. No
production constructor exists for either report outcome. Their only production handling was the
generic unresolved projection and their otherwise unreachable status labels.

The report enum and `VerifiedCurrentAffiliations` have no Serde implementation. The dataset marks
`verified_current_affiliations` with `serde(skip)`. The v1 frozen affiliation descriptor stores
profile binding, physical observations, archive inventory, and lineage, not outcome variants.
Its replay re-admits those retained observations through the same Reader. These inspected paths
are `export/qualification.rs`, `export/dataset.rs`, and
`export/qualification/current_affiliation/frozen/{descriptor.rs,capture.rs}` plus `frozen.rs`.
Source searches found no persisted report-state decoder or production constructor that would
give the removed variants historical meaning.

There is no descriptor/wire/schema/policy-version edit, migration, fallback decoder, or change to
the meaning of valid existing v1 captures. The four actual status strings are unchanged.
Unsupported report variants are removed rather than advertised as working production states.
The full generic domain outcomes remain available for their separate explicit contracts.

## Test contract changes, not defect-repair claims

The synthetic geography and selected-best refusal arrays now enumerate only actual unresolved
capture-day outcomes. They no longer claim coverage of runtime tenure-end or non-school states.
The seven-row audit fixture retains its original row and exclusion counts. Its two unsupported
synthetic outcomes become honest Unknown cases with `outside_capture_day` and `no_retained_row`
reasons and no admitted references. Assertions for those rows are intentionally changed to the
narrowed supported contract, not presented as repairs to previously correct product behavior.

The domain tests cover all four status/projection states, the four-state transition matrix for
ordinary/same/different/cooperative schools, all six evidence-arrival permutations, duplicate
references, contradiction persistence, required-admission withholding, and the existing
distinct-cooperative-program behavior.

The live/frozen parity tests cover actual capture-day resolution, unknown dates, co-op campuses,
cross-sport conflicts, conflict plus a missing same-day archive, selected-sports reference union,
nonselected-sport Unknown, and no-positive-sport Unknown. They compare live admission, frozen
capture, serialized v1 descriptor round-trip, and replay, including independent audit references,
unavailable reasons, and unresolved consumer exclusion. Reversing physical observation order and
per-observation evidence order must retain the same resolution and audit. Frozen dataset bytes
must remain unchanged during derivation.

## Separately identified test-fixture correction

The existing current-affiliation fixture advertised outdoor membership but emitted
`<div data-season-id="2"><svg class="icon-yes">`. The actual parser's season-cell expression
requires a `data-point` div class and `icon icon-yes`/`icon icon-no` SVG classes. Its
`active_seasons` function consumes the first three matching cells positionally as indoor,
outdoor, and cross-country; a season ID alone does not select the output slot. The previous
fixture therefore yielded no positive sport flags.

With Main's explicit direction, the fixture now emits all three matching cells in that order,
with only outdoor marked yes for existing callers. Existing intended behavioral assertions are
unchanged. A typed sport helper supplies the new cross-sport and no-positive-sport cases, using
the same capture/row/hash/metadata construction. There is no production parser change and no
parallel fallback fixture path.

## Remaining acceptance work

Main must run the repository's Moon format, check, lint, and relevant domain/report tests on the
integrated candidate. Static inspection and authored tests are not compiler or runtime evidence.
No full tenure, source-activation, non-school, or generic-affiliation integration claim follows
from this refactor. No new proof format or proof artifact is introduced.
