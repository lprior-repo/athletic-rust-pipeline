# TSSAA native directory collector

Tennessee Secondary School Athletic Association public school directory, postal claims and published
TF/XC appointments, plus sportless athletic directors.

Source: `https://portal.tssaa.org/common/directory/` (robots allows `/common/`).
Historical fixture corpus measurements: 456 indexed schools and 456 school detail captures;
430 pages with `staffPerson` rows, 26 without. These are not this run's acquisition counts.

## Entry points

| Function | Purpose |
|---|---|
| `parse_school_list(text)` | Parse 456 schools from the embedded typeahead array |
| `parse_school_page(text, school_id)` | Parse school entry + Track/XC/AD coach rows |
| `collect(ctx, options)` | Fetch the public directory, select indexed Tennessee schools, acquire details and commit source-owned schools, addresses, appointments and observations |

`Options` carries `limit`, `refresh`, `observed_on`, `states` and `school_names`. A nonempty state
selection must include Tennessee. Published non-Tennessee locations, including the indexed
Northpoint Christian School in Southaven, Mississippi, are excluded rather than relabelled.
School names use the shared canonical normalizer to select actual indexed records; absent or blank
requested names are explicit errors, never manufactured schools. Limits bound selected records,
including failed attempts, rather than the number of successes.

The collector uses the shared Fetcher for admission, pacing, one-attempt transport and immutable
cache capture retention. Detail acquisition is sequential. It does not access private APIs, log in,
solve challenges or synthesize appointments from membership/participation or `HasEmail` flags.

Detail URL ownership, indexed name and the published heading must agree. Public administration
tables use ordinary `<tr>` rows, so the athletic director is retained even without `staffPerson`.
TF/XC table roles, gender, raw published role/sport labels and available staff identifiers remain
attached to capture evidence. Unknown roles/genders are not promoted to head coach or mixed teams.
Only the public page's `mail_hide` arguments supply a decoded mailbox, then the shared domain
mailbox parser decides its kind. Mailing, physical and shipping postal claims remain separately
source-labelled, even when their addresses agree.

Current tenure requires the complete published staff-year statement naming this school, a
consecutive constrained academic-year range and an agreeing administration header after it.
The statement qualifies both administration and sport appointments for that published year;
the capture timestamp and run year never substitute for the statement. Preserve its exact
school-maintained-publication wording, URL, full capture digest and retrieval time.
An older year assesses as unknown for a different year, not former. “Retired Educator” is not
coaching-departure evidence. Missing year context retains unknown tenure; malformed, foreign or
contradictory context retains valid appointments with a parse error and no completion receipt.
Historical `tssaa_schools_v1` receipts do not certify this tenure-bearing projection.

School, coach, postal and source-observation freshness is the capture's actual `fetched_at`.
The caller's evaluation date does not replace acquisition freshness. Receipt-keyed
`tssaa_school_projection_v3` completions bind the capture URL, full digest and original acquisition
time; legacy owner-only keys do not certify this projection. Row effects are independently
idempotent through `AdapterContext`, supporting both store and recording routes.
Failed details, rejected index fields, postal claims and partial parses retain every diagnostic
and owed locator. Valid partial schools, appointments and immutable invalid postal captures remain
retained without a projection completion marker; identical replay reports the same unfinished
work without appending the valid rows again. Empty public indexes and requested names outside the
Tennessee selection are incomplete outcomes, not successful-empty discoveries.

Research journals and effect witnesses use distinct phases. The same research operation key
appears once in `staff_capture_research_v1` and once in `crawl_effect_receipts_v1`; comparing keys
without their phases falsely treats those two obligations as duplicate research effects.

## Retained regression evidence

The native regressions in `src/tssaa/tests.rs` and `src/tssaa/tests/collect/` consume the unchanged
retained public capture
`var/midwest-census/http/078e90211a0d5d3319742c16aa84949c.body`, whose metadata names
`https://portal.tssaa.org/common/directory/?id=157` and acquisition time
`2026-09-22T16:10:49Z`. It publishes Page High School, Franklin, TN 37064,
`6281 Arno Rd` (mailing/physical), `6281 Arno Rd.` (shipping), Benji Gray as athletic director,
Ron Brock/Ralph Ringstaff as boys'/girls' XC head/assistant coaches, and
Marcos Harris/Kevin Lewis/Adam Neelly as boys'/girls' TF head/assistant/assistant coaches.

Tests seed isolated offline Fetcher caches with those retained bytes, deriving full SHA256 through
the existing cache API; the historical metadata's 32-character `sha256` field is not promoted to
the domain's required 64-character postal capture digest. Historical captures are never rewritten.
This fixture replay is not fresh acquisition. Main's integrated tests and bounded live Page High
School acquisition/readback are dated in [verification evidence](../../../../docs/VERIFICATION-EVIDENCE.md);
they do not establish statewide or durable-workflow qualification.

## Fixtures

`crates/census-crawl/tests/fixtures/tssaa/`

- `directory_id3.html` — Alcoa High School detail page with 35 staffPerson rows
- `directory_id407.html` — Redemption School of Worship with no `staffPerson` rows; its public administration table still publishes an athletic director
- `robots.txt` — `Allow: /common`, `Disallow: /`
- `SOURCE.md` — byte digests and corpus measurements

## Run

```sh
env -u CI tools/moon-local run pipeline:xtask -- source-fixture tssaa
env -u CI tools/moon-local run pipeline:xtask -- source-test tssaa
```
