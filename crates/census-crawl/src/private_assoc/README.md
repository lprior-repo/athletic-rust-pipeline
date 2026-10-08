# `private_assoc` reader — association membership listings

## Purpose

Reads the membership listings the private-school associations publish and turns every listed school
into one weak `SchoolDirectoryEntry`: a `DirectoryKey::Weak` derived from the published name, city and
state, carrying `SourceLabel::PrivateAssociation { label }` and the address parts the listing
publishes. The module is parser-only — no descriptor, no fetch, no file I/O and no `AdapterContext`
writes (`NON_ADAPTERS`, ADR-020 §3). Its only caller is the corpus verb
`census-service school-address --associations <file>`, which hands it an operator-supplied file
(`crates/census-service/src/school_address/mod.rs`, `read_lane`).

Associations recognized, and the URLs the retired Python named for them
(the prototype tree's `hs-address-pipeline/private_associations_crawler.py`, constants at lines
16-18; ADR-020 §6 deletes that file from this tree, so read it from the prototype checkout or git
history):

|Association|URL the prototype used|
|---|---|
|NAIS, National Association of Independent Schools|`https://www.nais.org/members/schools/`|
|CAPE, Council for American Private Education|`https://www.cape-ed.org/schools/`|
|NASSP, National Association of Secondary School Principals|`https://www.nassp.org/schools/`|

## Evidence: no capture and no robots verdict

Searched on 2026-09-30, in this worktree and the research tree beside it:

- `census-prototype/raw/`, `census-prototype/data/`, `census-prototype/notes/` for
  `nais.org|cape-ed.org|nassp.org`: the only hits are other schools' pages linking an association,
  never a listing — `raw/labschool.org__475972c7d84d67baef346d0b` line 1015 (NAIS logo link) and
  `raw/vpaonline.org__b3458702c88b34dbbf4736bf` / `raw/www.vpaonline.org__88045bf963dfed25c93a74af`
  line 505 (NASSP links).
- `arh-python-port/{tools,research,hs-address-pipeline,notes}` for the same hosts: only
  `hs-address-pipeline/private_associations_crawler.py` lines 16-18 (the URL constants) and captures
  of unrelated sites (`research/sources/coach-directories-national/samples/dir/VT__membership.html`
  line 504, `research/sources/state-assoc-westcoast/samples/osaa-coaches.html` line 912). That path
  is the prototype tree's file; ADR-020 §6 deletes it from this tree, so a reader re-running the
  search finds it there or in git history, not here.
- robots: every `robots*` file under both trees belongs to another host (the hundreds of
  `research/sources/*/samples/robots-*.txt`), `coach-directories-national/tools/robots/` lists no
  NAIS/CAPE/NASSP host, and no `nais.org/robots.txt` / `cape-ed.org/robots.txt` /
  `nassp.org/robots.txt` string occurs anywhere under `/home/lewis/src/ad-law-scrape`.

So **no capture of a NAIS, CAPE or NASSP listing exists, and no robots verdict exists for any of the
three hosts.** `crates/census-crawl/tests/fixtures/private_assoc/` therefore holds no capture and no
`SOURCE.md`, and nothing may be hand-written into it (contract §3: byte-exact captures only). The
directory carries only its `README.md`, the contract file `cargo xtask replay` skips: `cargo xtask
source-fixture private_assoc` lists that one file, and `cargo xtask replay private_assoc` refuses
with `no captures under …: the directory holds no body to replay`. That refusal is the finding, not a
defect.

## Entry points

|Item|What it is|
|---|---|
|`SOURCE_ID`|`"private_assoc"`|
|`parse_listing(text)`|one listing body in; weak entries plus the skip/note ledger out|

## The shape this reader reads — unverified against a live response

The item shape is the Python's (`school_item.find('h3')`, `school_item.find('div',
class_='address')`); CAPE and NASSP were empty stubs there ("similar parsing logic"), so this reader
applies the NAIS shape to all three and assumes the sites share it:

- item: `<div class="school-list-item">` (the class token is matched case-insensitively)
- name: the first `<h3>` inside the item, entities decoded
- address: the first `<div class="address">` inside the item, `<br>` and commas splitting the parts;
  a trailing `ST ZIP` / `ST ZIP-1234` / bare ZIP group is taken as state and ZIP, the last
  digit-free part before it is the city, the first part is street line 1 and the rest street line 2
- association: the first recognized name in the body (NAIS / CAPE / NASSP, long form or abbreviation,
  case-insensitive and word-bounded)

Because no capture exists, none of this is verified against a live response, the reader has never
seen a real listing, and a real response may differ in any of these respects. An operator-supplied
file is the only reachable path; the first real capture must be committed byte-exact with `SOURCE.md`
and these selectors re-checked against it.

## Ledger rules

- **Skipped** (row dropped, line and field recorded): the item's `<h3>` is missing or empty
  (`school name is empty`), or the name reduces to no matching form (`matching name is empty`, e.g.
  `---`).
- **Noted** (row survives, field recorded): a published address part the domain refuses — a ZIP that
  is not five digits, a city over 64 characters, a street over 120 — and the parsed parts still land.
  A published state outside the census jurisdiction set also retains a note instead of disappearing.
- **Refused** (`CrawlError::Invariant`): the body carries no `school-list-item` element, or it names
  no association this reader recognizes; the reader names the three it accepts rather than guessing.

Each published part is validated on its own (`StreetLine::parse`, `CityName::parse`, `ZipCode::of`,
`census_state`) instead of through `directory::postal_address`: that helper drops the whole address on
the first unusable part, and a listing's ZIP is the part most likely to be malformed, so routing it
through `postal_address` would cost a row its street lines and its key's city/state for one bad
digit. Here one bad part costs exactly one ledger note and the row keeps everything else.

The reader admits at most 8 MiB of source body, 20,000 listing items, 16 KiB per item,
128 address segments and 1,024 bytes per decoded name or address field. Text and segment
growth is reserved fallibly before appending. The shared `ReadOutcome` also enforces
entry/issue counts, issue detail length and conservative aggregate retained bytes.
Fallible row and ledger calls propagate admission failures to `ReadOutcome::stop`, retaining
the accepted prefix and the rejected locator even when another ledger row cannot fit.
Whole-body refusal returns an error, never a successful empty directory.

Neither the unqualified selectors nor exhaustion of the supplied file proves a finite
association membership frontier. The reader therefore does not call `finish`: clean
parses remain `Unknown`, and skipped rows, notes or stopped admission remain `Partial`.


## Fixtures

Documentation only. `crates/census-crawl/tests/fixtures/private_assoc/` carries no captures (see
Evidence) and its README records what a future capture must satisfy. The reader's behaviour is
asserted from two places: the in-module tests in `tests.rs` over small bodies whose selectors come
from the Python, and the census-service lane
`crates/census-service/tests/private_assoc_directory_properties.rs`, which drives the same public
entry point end to end.

## Commands

```console
tools/moon-local run pipeline:tests -- -E 'test(private_assoc)'
env -u CI tools/moon-local run pipeline:xtask -- source-fixture private_assoc
env -u CI tools/moon-local run pipeline:xtask -- replay private_assoc
```

## Before this adapter lands

- [x] evidence recorded: no capture and no robots verdict, with the paths searched
- [x] reader and ledger asserted (in-module `tests.rs`, service-side `private_assoc_directory_properties`)
- [x] parser-only placement: no descriptor, no fetch, no `AdapterContext` write; the corpus verb is the
      only caller
- [ ] byte-exact capture plus `SOURCE.md` — blocked: neither research tree holds a capture of any of
      the three hosts, so the fixtures directory stays documentation-only until a real response is
      captured outside the process
