# `deepen.py` stage-two semantics vs the Rust run path — 2026-09-29

The prototype's generic stage-two crawler (`census-prototype/deepen.py`, 154 lines) is the path that
produced the `out/extra/<label>.jsonl` records the DragonFly summary lane is merged from, so its
semantics are part of the port's contract. This is a source-and-execution audit: the Python file was
read end to end, and the Rust side is `crates/census-crawl/src/coach_directories/collect.rs` (read,
with its executed evidence in `docs/VERIFICATION-EVIDENCE.md`).

## What `deepen.py` does

| Step | Behaviour |
|---|---|
| Input rows | `out/schools.jsonl` (stage one), filtered by `--state` (upper-cased) |
| URL selection | every non-empty value of `--url-field` (comma-separated fields), de-duplicated in first-seen order; `--urls-file` and `--only-urls` bypass the table; `--limit` truncates after de-duplication |
| Join | per URL, `{name, association_id, state}` taken from the stage-one row that named it |
| Fetch | `Fetcher` (pacing, robots, cache); `--xhr` adds `X-Requested-With: XMLHttpRequest`, `--refresh` bypasses the cache, `--offline` is cache-only |
| Concurrency | `ThreadPoolExecutor(max_workers=args.workers)`, default 8, preserving result order |
| Per-URL record | `{url, status, bytes, join, schools, coaches}` on success |
| Failure records | `{url, status: HTTP code or exception class name, error: str(error)[:200]}`; an empty body is `status: "empty"`; a parser exception is `status: "parse-failed"` with `"ClassName: message"[:200]` |
| Output | `out/extra/<label>.jsonl`, one JSON object per line, `ensure_ascii=False`, no trailing newline |
| Summary line | `{label}: {n} URLs, {ok} ok, {schools} school records, {coaches} coach records, {seconds}s -> {path}` |

Merge-side rules that consume those records live in `run.py:154-197` (`load_extras`):

* a record whose `status` is not `"ok"` is **skipped entirely** — failures stay in `extra/` as
  evidence but never reach the artifact;
* the school row's `name` is **overwritten** with the stage-one (`join`) name, so the detail page's
  own spelling is discarded, and `join.association_id` fills `association_id` when the summary did
  not carry one;
* coaches take `school = join_name` and, when their own `state` is empty, `join.state` or the
  two-letter prefix of the label file name;
* the varsity-only filter runs here (`(level or "Varsity") == "Varsity"`), i.e. after the parse and
  before the merge de-duplication.

## Rust counterpart and the mapping

| `deepen.py` | Rust (`coach_directories`) |
|---|---|
| stage-one rows from `out/schools.jsonl` | directory pages fetched by `collect` (registry host, 1 request/s, one in flight) |
| `--url-field detail_url` | `summary_url(short_code)`, derived from the directory row |
| join `{name, association_id, state}` | the directory row already minted the school (`CanonicalSchool::new` + `association_school` identity); the summary is absorbed into that entity |
| failure records with a status and a 200-char error | `fetch`/`parse` failures increment `AdapterReport::errors` and add a note; the school is still written without coach rows |
| skipped non-`ok` records | equivalent: a failed summary contributes no coach rows (the school row comes from the directory either way) |
| `join.name` overwrites the detail name | SUPERSEDE: `absorb_summary` keeps the directory name and adds the summary's spelling as an **alias** (`map.rs:94-105`), so both spellings survive instead of one being dropped |
| varsity-only filter at merge | the same rule, at emission (`row_hygiene::is_varsity_level`, `EmissionScope::Census`) |
| `out/extra/<label>.jsonl` | store tables (`Schools`, `Coaches`, `SourceObservations`) plus a per-school journal entry (`JOURNAL`, key `state:short_code`) |
| `--xhr` | not needed by this lane (the API answers without it); `FetchOptions` carries headers for adapters that do need it |
| `--limit` / `--only-urls` / `--urls-file` | **no counterpart**: `Options` selects by state and by school name only |
| 8 worker threads | the Rust run path is strictly sequential (`collect.rs:99-107`, `152`, `187` await each fetch in turn) |

## Findings

1. **Failure policy matches, and is better instrumented.** Both sides treat a failed detail page as
   data and keep going; Python writes a failure record to `extra/` and skips it at merge, Rust counts
   `errors` and notes the URL on the run report while still writing the school row. No port gap.
2. **The name-join rule is a recorded supersede, not a gap.** `load_extras` overwrites the detail
   page's school name with the stage-one name; the Rust lane keeps the directory name and unions the
   summary's spelling into `aliases` (ADR-016 S07 already governs alias union). Nothing is lost.
3. **Sequential fetching is not a throughput regression at the registry's rate.** `deepen.py`'s 8
   workers overlap network latency, but the polite fetcher paces one request per second per host, so
   the request budget — not the worker count — sets the wall clock; `census-crawl`'s loop and the
   prototype's pool both issue ~1 request/s per host. Parallelism only becomes a difference if the
   registry rate or host count rises, and `registry.rs` records each host's rate and in-flight bound
   for that case.
4. **Gap: no per-run URL or count override.** `--limit` and `--only-urls`/`--urls-file` have no Rust
   equivalent, which the integration-smoke work will need (a bounded subset run against the live
   host). The natural home is an `Options::max_schools` (or explicit school-code list) applied inside
   `process_school`, checked before the summary fetch so a limit costs no requests. Not implemented
   here; recorded as a gap for the run-path slice.
5. **Gap: failure records are notes, not rows.** Python's per-URL failure record is structured
   evidence (`status`, `error`, `bytes`); the Rust equivalent is a formatted note string on
   `AdapterReport`. For the probe lane the same information is a structured record
   (`ProbeRecord::status`/`error`); the collect lane has no such table. This matters only if a later
   audit has to count failures by kind without parsing prose — recorded, not changed, because adding a
   store table is an interface decision for the owner of the adapter-report contract.

## Limits

Source-and-evidence audit: no prototype `deepen.py` run was executed for this note (its semantics are
read from the file and from the merge code that consumes its output), and the Rust side was not
re-run for this note beyond the lane's committed tests. `bound_crawl.py` also writes `out/extra`
records; it is not covered here.
