# School-sites long-tail Source Report

Source token: none (queue-commissioned host family, not a registry source)
Lane slug: `school_sites` — `census-service school-sites` over `census-crawl::school_sites`
Coverage: any school website supplied by an owner-approved queue (48 states + D.C. queue inputs)

The canonical CLI acquisition contract added during the 2026-10-07 repair is
specified below. The earlier fragment-only observations and acceptance chain
are historical evidence, not verification of that cutover.

## Why this lane exists

Every directory, association and platform lane leaves schools whose only published coach/AD
evidence is their own website. The prototype measured this gap: `census-prototype/school_sites.py`
crawled 20,859 school sites with a rendering browser and `out/school_sites/` holds 803 artifacts
with at least one coach hit. This lane is the same crawl as serviced, cache-backed acquisition with
explicit provenance, so those contacts can travel the proof pipeline instead of a script's output.

## Input contract

Queue JSONL, one record per school: `{"state": ..., "name": ..., "website": ...}`. Extra keys are
ignored, so the NCES-recovered queues (`var/school-site-wave4-20261005/*.jsonl`, 3,016 schools) and
`var/school-address-join-20261004` records feed it unchanged. `state` resolves through
`UsJurisdiction::parse` (code or full name); records without a website, with an unmappable
jurisdiction, or duplicating an earlier `(state, school)` are counted and skipped, never crawled.
`--state` supplies a jurisdiction for queues that carry none; `--sample` takes an evenly spaced
slice before `--limit` truncates. Site artifacts are resumed by file presence; `--refresh` refetches.

## Politeness and admission

One `Fetcher` for the run: 1,000 ms default per-host delay (`--delay-ms`), per-host `robots.txt`
enforced with the 64 KiB cap, per-origin advisory `flock` so a second process cannot double a host's
rate, and the response cache under the store's HTTP root, which is also what `verify-coaches`
reads. There is no registry descriptor: the host set is per-school and unbounded, so acquisition
authority is the queue itself.

School hosts redirect constantly (`http`→`https`, apex→`www`, school→district/platform), and a
cross-origin redirect is refused without a grant. `--authorize-queue-hosts` grants the base domain
of every host in the queue for that collection; granted hosts stay capped at 2 rps and every other
check (private address, robots, pacing) still binds. The flag is the operator's statement that the
queued sites were commissioned, matching `verify-coaches --authorize-cited-hosts`.

## Extraction

The rules are the prototype's, recompiled in Rust: `mailto:` and bare-address harvest, six coach
phrase patterns and three athletic-director patterns over a flattened page with a 60-character
window, a junk-context filter, `Sport`/`CoachRole`/`Gender` classification, the `Sport | Coach |
E-mail` table shape, and the follow policy (ranked link pool capped at 7 analysed pages, three
guessed paths at 10, WordPress search plus three followed hits at 14). `school_sites::tests`
covers each. The extraction lives in `school_sites/parse/{patterns,rules,text,hits,urls}.rs` and
the fragment projection in `school_sites/contacts/{mod,vocabulary}.rs`, each file inside the
repository's 300-line budget with every function inside its 60-line budget. Every fetched page
records `url`, SHA-256 content digest, `fetched_at` and status in the site artifact, so a fragment
cites bytes a later gate can re-read from cache.

Fragments are candidates, never contacts: the twelve-column contact shape with
`verified_proof_digest` empty. `verify-coaches` re-fetches each `source_url`, writes the proof, and
its `<STATE>.csv` union is what `merge-coaches` accepts. The lane writes no store state.

## Artifacts

`<store>/out/school-sites/<STATE>__<school-slug>.json` — prototype-compatible signals
(`state`, `school`, `website`, `emails`, `coach_hits`, `ad_hits`, `pages`) plus `page_evidence`;
`"empty": true` when a site answered but published nothing, and no file at all when the homepage
failed. `<out>/fragments/<STATE>.csv` — the candidate rows. `<out>/report.json` — counters and the
per-site failure list with its fetch error.

## Measured behaviour (2026-10-05, this workstation, live network)

| run | queue | planned | crawled | empty | failed | emails | coach rows | AD rows |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| wave-4 head | 12 (MO/CT/DE) | 12 | 7 | 6 | 5 | 3 | 0 | 0 |
| prototype-URL probe | 10 | 10 | 9 | 1 | 1 | 118 | 9 | 0 |
| prototype-URL probe 2 | 12 | 10 | 10 | 1 | 0 | 259 | 16 | 4 |
| wave-4 slice | 40 (TN/GA/ND/NM/MS/MO) | 40 | 24 | 13 | 16 | 53 | 0 | 2 |
| wave-5 high-school slice | 36 (AL/AZ/CA/CO/FL) | 36 | 15 | 4 | 21 | 160 | 3 | 4 |

The high-school population is the coach-bearing one: on the wave-5 slice the crawl emitted three
coach candidates and four director candidates from fifteen crawled schools, the gate shipped the
CA Loyola athletic director row (`ok`) and refused the rest as `render-required`, and
`merge-coaches` kept one row. High-school queues also fail more often on stale links — 21 of 36
hosts did not answer.

Probe 1 re-crawled the exact URLs the prototype recorded as carrying coach hits. Three sites
reproduce the prototype's signals exactly (`cacmustangs.org`: 2 coach hits, 109 emails;
`altavistahs.com`: 4/1/2; `cherokeek12.org`: identical names). Two platform sites the browser saw
(`gophslions.com/staff`, `cullmanhigh.cullmancats.net`) yield nothing through a static read: those
pages build their staff list client-side. Probe 2's 20 fragment rows all came back
`render-required` from the gate, and probe 1's AR and AZ rows likewise — the crawler finds names in
JSON payloads a text-level gate cannot corroborate, which is exactly the refusal the gate exists to
make. Failed sites are stale CCD links (404) and dead hosts (DNS), reported individually.

## Acceptance chain (executed)

```text
target/debug/census-service --store var/school-site-wave4-20261005/smoke-store \
  school-sites var/school-site-wave4-slice-20261005.jsonl --authorize-queue-hosts \
  --out var/school-site-wave4-20261005/smoke-out
  -> planned=40 crawled=24 empty=13 skipped=0 failed=16 emails=53 coach_contacts=0 ad_contacts=2

target/debug/census-service verify-coaches \
  --fragments var/school-site-wave4-20261005/smoke-out/fragments \
  --cache-dir var/school-site-wave4-20261005/smoke-store/http --authorize-cited-hosts \
  --out var/school-site-wave4-20261005/verify --union var/school-site-wave4-20261005/union
  -> verified fragments: 2 files, 2 rows, 1 shipped
  -> TN row verdict `ok` (Emily Hopper, Athletic Director, aes.mcnairycountyschools.com/apps/staff)

target/debug/census-service merge-coaches --fragments var/school-site-wave4-20261005/union \
  --out var/school-site-wave4-20261005/coach-contacts.csv --report var/school-site-wave4-20261005/merge.md
  -> kept 1 rows from 1 states; rejected 0; AD rows 1

census-service school-sites var/school-site-wave5-slice-20261005.jsonl --authorize-queue-hosts \
  --out var/school-site-wave5-20261005/out
  -> planned=36 crawled=15 empty=4 skipped=0 failed=21 emails=160 coach_contacts=3 ad_contacts=4
census-service verify-coaches --fragments .../wave5-20261005/out/fragments \
  --cache-dir .../wave5-20261005/store/http --authorize-cited-hosts --out .../verify --union .../union
  -> verified fragments: 2 files, 7 rows, 1 shipped   (CA `ok`, six `render_required`)
census-service merge-coaches --fragments .../wave5-20261005/union \
  --out .../wave5-20261005/coach-contacts.csv --report .../wave5-20261005/merge.md
  -> kept 1 rows from 1 states; rejected 0; AD rows 1
```

Unit coverage: `cargo test -p census-service --lib school_sites` (7 tests: person cleaning, contact
rows with page provenance, queue filtering/artifact naming, even sampling, unmapped states, fragment
header, prototype-compatible artifact keys) and `cargo test -p census-crawl --lib school_sites`.
Refactor equivalence: the same wave-5 slice re-crawled after the module split produced byte-identical
`fragments/CA.csv` and `fragments/FL.csv` to the pre-split run (the per-site JSON differs only in
`fetched_at` and in pages whose live links changed between runs).

## Limits and next work

- Static reads only. Rendering-dependent directories need the browser lane
  (`census-serve --browser-profile` plus `Fetcher::with_browser_lane`); the gate already separates
  those rows as `render-required` instead of shipping them.
- Crawl-side name capture still yields junk candidates (`Brown Library Media`,
  `Of The Year July`) that pass phrase patterns. They ship nowhere because the gate requires the
  value and its role in one block, but they inflate the review surface. A structured-markup path
  (table cells, `Person`/`JobTitle` microdata) would raise the shipped share.
- The wave-4 population is largely elementary and middle schools: its yield is published e-mail,
  not coaches. The wave-5 high-school slice yielded three coach candidates and four director
  candidates from fifteen crawled schools, of which the gate shipped one director row; the rest were
  rendering-dependent. Coach-bearing queues should come from the high-school universe, and the
  shipped share depends on the gate, not the crawl.

## Canonical acquisition cutover (2026-10-07; verification pending)

`school_sites::run(fetcher, args, store)` now consumes the queue only as a
selector over existing public-source-discovered canonical schools. Exact parsed
official school website, jurisdiction, and canonical name or canonical alias
must identify one owner. Missing or ambiguous owners are explicit failures and
remain owed; queue labels never mint a school. Source identity and fetched or
parsed public-source evidence must already exist on the matched school.

Ownership lookup retains at most 64 selected owners and 8 MiB of serialized
school data per window. Queue input is admitted at 8 MiB, 65,536 rows, and bounded
name/website/jurisdiction fields; admission and counters are fallible.
The run season comes from the bound store manifest, otherwise `SchoolYear::DEFAULT`.
Acquisition uses `census_crawl::school_sites::collect_contacts`, not fragment
guesses or WordPress search. Artifact presence never bypasses canonical research.

School-office and athletics-office mailboxes remain independent of named coach
and athletic-director claims. Every acquired page binds its actual locator,
SHA-256 and physical capture time. Missing or invalid capture dates and stale
captures remain owed. Sparse new-claim/research deltas, their journal marker and
effect travel through the recording-aware adapter batch. Earlier admitted
canonical prefixes survive later refused or incomplete pages.

The report's failures, failed count and error count now include canonical
ownership/acquisition/research debt, allowing the CLI to reject incomplete work.
Its crawled count measures processed canonical acquisitions; emails measures
selected office mailbox purposes; empty means a completed acquisition with no
selected office mailbox. Fragment orchestration and the obsolete fragments
report field are removed. `census_crawl::school_sites::crawl_site` and
`contact_rows` remain distinct candidate-artifact APIs; they cannot grant
canonical claims or veto successful canonical acquisition through guessed routes.

The new offline service scenarios use the existing verbatim CAC contact extract
with explicitly controlled cache metadata. They cover canonical-alias ownership,
unmatched queue population refusal, capture-bound mailbox persistence, and a
bound later run year leaving the physical 2026 capture stale. They do not assert
a live 2026 fetch. No build, test, formatter, lint or runtime command was executed
by this concurrent repair worker; the owning agent must run the integrated
compiler checkpoint and acceptance lanes before certification.
