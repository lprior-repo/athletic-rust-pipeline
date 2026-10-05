# Tap plan - every resource in the 2026-10-04 coach-source bundle

Inputs (moved into the repo 2026-10-04):

| Input | What it is |
| --- | --- |
| `COACH-SCHOOL-RESOURCES-COMPLETE.json` | 278 sources; 25 requested states; 75 prioritized (3 per state) entries |
| `ALL-STATE-COACH-AND-SCHOOL-RESOURCES.md` | Human-readable companion report |
| `TAP-REGISTRY.json` | One disposition + staffing entry per source (generated 2026-10-04) |
| `research/sources/coach-directories-national/coverage.json` | Repo's 51-jurisdiction tier-1 survey (field inventory, robots, render behavior) |

Beads: epic `athletic-rust-pipeline-6ec`; children `.1` (wave 0), `.2`-`.5` (wave 1), `.6` (wave 2 backlog).
Fixed on this path: `[bug] athletic-rust-pipeline-0fp` - `xtask new-source` emitted non-compiling
modules (dev-only `anyhow`, `census_crawl::` self-path); templates repaired, `cargo check -p
census-crawl` clean, both wave-1 adapters scaffolded from the fixed tool.

## Definition of a tap (acceptance)

A resource counts as tapped only when all five exist:

1. **Capture evidence** - raw bytes as served under `probes|extract|captures`, with a manifest
   (url, http status, bytes, sha256, fetched_at, robots status).
2. **Derivation** - rows in the `CoachContactRow` CSV schema (see
   `crates/census-crawl/tests/fixtures/coach_contacts_sample.csv`) or an adapter run report; every
   row traceable to a capture fragment.
3. **Import** - `census-service --store <root> import-coaches <csv> --observed-on <date>` for
   extracted rows, or `census-service --store <root> provider <adapter> ...` for adapters.
4. **Readback** - coaches visible for the state (`census-service verify-coaches`, workbook
   `Coaches` sheet); counts recorded.
5. **Dated evidence** in `docs/VERIFICATION-EVIDENCE.md` with exact commands and limits.

No capture, no row. Robots first; <=1 req/s; anonymous access only.

## Registry dispositions (278)

| Disposition | Count | Mechanism |
| --- | --- | --- |
| `seed_only` | 90 | school identity/addresses (state ED, participation, private inventories) - feeds the school corpus, not coach rows |
| `qualify_probe` | 84 | live probe required before a mechanism is chosen |
| `extract_rows` | 51 | structured access (CSV/JSON/API/PDF/sheet) with coach/email fields -> `coach_contacts` import |
| `adapter_or_extract` | 36 | priority association directories; qualify then adapter or extraction |
| `existing_adapter` | 10 | registered adapter already covers the host (CT/OH/IL/PA/KS) |
| `platform` | 7 | Home Campus (CA/FL/NJ), GoBound (IA/AZ/SD), MaxPreps (national) |

## Wave 0 - registered adapters (`6ec.1`, owner Main)

SRC-076 CT (`ciac` via coach_directories), SRC-098/099 OH (`ohsaa_portal`), SRC-120/121/124/125 IL
(`ihsa`), SRC-229 PA (`pa_piaa`), SRC-240/242 KS (`ks`). Run each against a scratch store copy and
read coaches back, or record the named refusal.

### Wave 0 results (2026-10-04, store `var/tap-wave0-20261004`)

| Source | Result | Follow-up |
| --- | --- | --- |
| RI smoke + `export-data` | 55 schools / 258 coaches read back; loop verified | - |
| KS | 3 schools, 3 AD emails | expand limit in wave 2 |
| IL | 58 with email; 6 reveal errors (429 then host cooldown) | cooldown policy working; retry later |
| PA | 154-school smoke → full-state run: 1456 schools, 1529 AD rows, 0 errors | closed |
| OH | 784 names derived from the SRC-097 seed feed the adapter, then the tap meets a TLS wall: `officials.myohsaa.org` offers only TLS 1.2 CBC suites and resets TLS 1.3; the rustls client is AEAD-only | named transport refusal; needs a CBC-capable client decision or the browser lane |
| CT | `ciacsports.com` cert expired 2021; DragonFly `states/CIAC` API live (200, 2 pages) | CIAC in `ASSOCIATIONS` but not `REGISTERED`: survey-qualify then register |

**Wave 0 follow-up (2026-10-04).** Three rows are now verified end-to-end and their beads closed:
PA (`pa_piaa` with `--authorized-host www.piaa.org`; 154-school smoke, full run recorded in the
evidence ledger), CT (`ciac` repointed to the live FusionPoint host `ciac.fpsports.org`; 184
schools / 1 033 TF/XC coach rows) and KS (whole KSHSAA membership in one 489 KB API response; 526
schools / 526 AD emails). IL is partial by provider rate limiting — the API answers 429 after about
a dozen requests and the recorded cooldown then refuses the rest, so one school completes per run;
its beads stay open under "retry later". OH's school-name gap was closed by deriving 784 names from
the in-bundle SRC-097 enrollment figures (sha256 `f3127f83…`), but the tap then met a host-level TLS
incompatibility (CBC-only server vs the rustls AEAD-only client) recorded as a named transport
refusal in `probes/oh-ohsaa/manifest.json`. Byte-level captures and
sha256 manifests live in `probes/{ciac-fpsports,pa-piaa,ks-kshsaa,il-ihsa,oh-ohsaa}/`, with the
dated section in `docs/VERIFICATION-EVIDENCE.md`.

## Wave 1 - staffed 2026-10-04

| Bead | Owner | Slice | Deliverable |
| --- | --- | --- | --- |
| `6ec.2` | deepseek-flash | Home Campus SRC-017/034/096; GoBound SRC-129/161/224 | `probes/{home_campus,gobound}/` manifest + captures + FINDINGS.md with endpoints and sample records |
| `6ec.3` | deepseek-flash | MA SRC-068, NY SRC-054, SD SRC-222, NV SRC-175 | `extract/<id>/` captures + `rows.csv` (exact header) + REPORT.md |
| `6ec.4` | gpu5090-coder | AZ SRC-155 AIA | `crates/census-crawl/src/aia/` + fixtures + `research/sources/aia/SOURCE_REPORT.md` |
| `6ec.5` | gpu3090-coder | UT SRC-164 UHSAA | `crates/census-crawl/src/uhsaa/` + fixtures + `research/sources/uhsaa/SOURCE_REPORT.md` |

Main owns shared wiring for new adapters: registry descriptor, applicability table, provider arm,
then compile and fixture-test.

**Wave 1 results (2026-10-04).** `6ec.2` probes are verified by Main: Home Campus search and
school-details JSON answer 200 with the browser-equivalent `X-Requested-With: XMLHttpRequest` +
`Referer` headers (CA 1,727 / FL 881 / NJ 453 directory entries; coach JSON carries
sport/role/name/email), so SRC-017/034/096 move on to an extraction/adapter slice; GoBound answers
403 on every tenant route and its robots disallows `/api/` and the directory families, so
SRC-129/161/224 close as a platform refusal with substitutes named. `6ec.3` (MA/NY/SD/NV extraction),
`6ec.4` (`aia` AZ) and `6ec.5` (`uhsaa` UT) landed with their own dated evidence.

## Wave 2+ - backlog (`6ec.6`)

Every one of the 256 unstaffed sources has its own bead under `6ec.6` (created 2026-10-04), and
`research/sources/coach-coverage-bundle-20261004/TAP-REGISTRY.json`'s `staffing.bead` names that
bead per source. Each bead carries the registry's URL, host, category, disposition, mechanism,
survey fields and note, the five-point acceptance above, and the capture constraints.

Select work with Beads, not this file:

```bash
bd list --label tap-program              # all per-source beads
bd list --label wave:2                   # wave-2 sources
bd list --label kind:qualify_probe       # probe-first sources
bd list --label state:TX                 # one state's sources
bd ready                                 # unblocked queue, priority order
```

Priority order is the registry's `priority_norm` (P0/P1 first), then state batches. A source is
done only when its bead is closed against the five-point acceptance with dated
`docs/VERIFICATION-EVIDENCE.md` evidence.

## Store and readback path

- Import rows: `census-service --store <root> import-coaches <csv> --observed-on <date>`
- Run adapter: `census-service --store <root> provider <adapter> [--limit N] [--states ...]`
- Read back: `census-service --store <root> verify-coaches`; workbook build via `census-service workbook` / `census-report`
- No national census store survives the 2026-10-04 cleanup. The school-identity base for readback
  scratch stores is the NCES CCD+PSS corpus generation `var/school-address-join-20261004/corpus-web`
  (address-join run, 2026-10-04). The historical workbook in `~/Downloads` is preserved evidence,
  not fresh acceptance input; historical stores are never reopened as fresh-run evidence.

## Limits

- `opened_verified` in the bundle means the page was inspected, not that child pages, exports or
  current coach tenure were verified; every tap re-verifies live at capture time.
- JS platforms (Home Campus, GoBound) may refuse anonymous requests; a documented refusal with
  response evidence is an acceptable probe outcome and downgrades the slice to the browser lane.
- 90 seed sources deliberately produce no coach rows; they feed school identity and addresses.
