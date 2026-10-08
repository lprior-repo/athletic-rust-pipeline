# Arbiter organisation adapter

Associations that delegate their member directory to Arbiter embed `live.arbiter.io/org/<id>`.
The organisation API behind the embed carries member schools and their coach rows, which is the
largest measured yield of the port queue (2,932 coach rows across the four records). The adapter walks it end to end: mint a token, page the member list, then read one
coach page per school.

## Hosts and robots

- `https://live.arbiter.io/directory/` — the stable public SPA entry. The adapter GETs its
  HTML through the existing Fetcher, selects exactly one distinct active external module,
  then GETs that declared bundle through the same Fetcher. Build asset names are discovered,
  never pinned, guessed, probed or used as fallbacks. The public client credential literals
  consumed by the existing parser are in that bundle. Credential/token values are not logged.
  The retained robots observation is a SPA shell with no directives.
- `token.arbitersports.com/connect/token` — OpenID token endpoint, `POST` form
  `client_id`/`client_secret`/`grant_type=client_credentials`/`scope=Registration`, answers
  `{access_token, expires_in, token_type}`. `robots.txt` answers `404`, i.e. allow-all.
- `services.arbitersports.com/api/v2` — the gateway. Every path answers `401` without a Bearer
  token, and `robots.txt` answers `404`, i.e. allow-all.

Association-hosted Arbiter sites such as `ossaa.arbitersports.com` refuse every user agent and are
not read: the lane uses the API hosts only.

### Entry discovery and ownership

Discovery tokenizes at most 65,536 UTF-8 HTML bytes, with at most 8,192 emitted tag/comment/
doctype tokens, 128 active script tags, 32 active base tags, 128 nested inert containers and
4,096 decoded URL bytes per attribute or resolved URL. Those are adapter parsing limits; transport
still uses the shared Fetcher's body, timeout, admission and redirect limits. No private HTTP client or
additional retry is introduced.

The html5gum emitter decodes attributes, honors their first occurrences and excludes comments,
templates, foreign SVG/MathML markup and raw text (including noscript).

Inert containers use matched bounded scopes: an unrelated foreign closing tag cannot activate
an excluded declaration. Self-closing foreign elements do not retain a scope; HTML template
and script self-closing slashes do not acquire XML closing semantics.

Inline modules, preloads and classic scripts are not entry candidates. The first active `base[href]` is
authoritative in document order; it affects later declarations only. Later bases cannot
override it. Identical resolved module declarations are one candidate; distinct candidates
fail before any bundle dispatch. Missing candidates, truncated tags/scripts, invalid UTF-8,
oversized attributes and unsafe URLs are typed schema failures, not empty-directory success.

Resolution uses the actual observed response URL when present. Historical captures without
that metadata resolve against the requested URL while retaining `response_url: None`;
discovery never invents an observed final URL. Document, base and module must remain on the
stable entry's credential-free origin. Production's fixed authority is HTTPS
`live.arbiter.io` with the default HTTPS port; HTTP downgrade, alternate hosts/ports and userinfo
are refused. Modules must be direct nonempty `.js` assets under `/directory/assets/`, without
encoded filename bytes, subdirectories, queries or fragments. Observed bundle response URLs
are checked against the same ownership/path constraints before credentials are consumed.

Both GETs preserve `options.refresh || ctx.refresh`. A cached shell can still name a deleted
asset: that fetch failure stays explicit until an ordinary caller-requested refresh.
The token POST always has `refresh = true`, so cached tokens are never replayed. Entry, bundle
and token Fetch errors keep their original typed retry disposition; only Restate owns retries.
No module graph traversal, import evaluation, source-map request or credential probing occurs.

## Endpoints

- Members: `GET /organization/public/{org}/children?&pageSize=200&pageNumber={n}`. Returns
  `{data:{total, rows[]}}`; a row carries `publicId`, internal `id`, `name`,
  `phoneNumber`, `enrollmentCount`, and an embedded `primaryContact` with `firstName`,
  `lastName`, `roleName`.
- Coaches: `GET /legacy/public/{org}/coaches?filter.EntityId={publicId}&&pageSize=200&pageNumber={n}`
  for one school, or without the filter for the whole organisation. A row carries `firstName`,
  `lastName`, `coachPositionName` (`Head Coach`, `Assistant Coach`, `Associate Head Coach`, …),
  `sportName` (`Cross Country, Boys`, `Track & Field - Indoor, Girls`, …), `levelName`
  (`Boys Varsity`, `Coed Middle School`, …) and `roleName`.

Both endpoints paginate on `pageNumber`; the walk ends on the first page that returns fewer rows
than `pageSize` (an empty page included), not on the offset against `total`. A page that is full but
answers without a usable `total` therefore keeps the walk going instead of stopping early, and a
short page whose response still reports more rows than have been read — the server contradicting
itself, which the walk detects on the rows actually received rather than on the page window — is
recorded as a failure note rather than passing as the end of the list. Each member page is mapped
and journaled as it is read, so the walk's working set is one page rather than the whole member
list. A member walk, or a per-school coach walk, stopped by the 64-page bound records a failure
note too, so no truncation is silent. A state named twice in the `--states` list is walked once.

## Organisations

| Organisation | State | Member schools |
|---|---|---|
| NHIAA, org `2132` | New Hampshire | 89 |
| KHSAA, org `2507` | Kentucky | 489 |
| MHSA, org `4497` | Montana | 215 |
| WVSSAC, org `4223` | West Virginia | 269 |

Org `106940` was the queue's Oklahoma candidate and answers an empty member page (re-checked
2026-09-29), so Oklahoma stays uncovered by this lane.

## Mapping rules

Only Cross Country and Track sports become rows; a sport label carrying `ski` is dropped before
the shared label table sees it (Arbiter's `Cross Country Skiing` would otherwise read as Cross
Country). The label table is the measured one the prototype's exact-key table was replaced with,
so `Track & Field - Indoor` and `Track & Field - Outdoor` map to indoor and outdoor Track instead
of being dropped.

The side of a coach comes from the sport label when it states one and from the level label
otherwise: NHIAA writes `Cross Country, Boys` for some rows and `Boys Varsity` for the rest, so
both are read and a row that states neither becomes `Gender::Mixed`.

Role comes from the position label through the shared table, which keeps head and assistant coaches
and drops the non-coaching titles (`Volunteer Coach`, `Secretary`, …) the API also returns. The
table matches on substrings, so `Associate Head Coach` — one NHIAA row, which the prototype's
exact-key role map drops — is kept as `HeadCoach`.
Athletic directors are minted only from the member row's `primaryContact`, with no sport, because
the coach endpoint does not carry them; a contact whose role name is not a coach role is dropped
rather than minted as a director, and the member row itself is still written (see below).

A row whose person name is empty, whose position is not an identifiable coaching role, whose sport
is outside Cross Country and Track, or whose level label states a division other than varsity (`Coed
Unified`, `Coed Middle School`, `Boys JV` — matched as a substring, so `Varsity` and `- Varsity`
both count) is dropped without a coach row and without an absence claim: the endpoint is partial
across sports by design, so a missing row is not evidence of no coach.

Every member row with a name is written as a canonical school plus a source observation keyed by
the source namespace; the contact only decides whether an athletic-director coach row is written
alongside it. Coach completion is keyed by jurisdiction, organisation and public school owner,
not by the canonical school's name-derived id. Re-evaluation can read completed pages from the
immutable cache without issuing physical requests or duplicating their effects. Incomplete response
markers bind that exact public owner and capture; ordinary recovery reacquires only a cached response
known to be incomplete, retaining earlier valid rows and the failed capture archive.

A member page that fails or does not parse, and a coach page that fails, are recorded as failures
and end only that walk — the remaining organisations in `--states` still run — while entry
discovery, bundle acquisition or token failure ends the run before an organisation is walked.
Corrected code does not replay retained terminal failures, mutate old captures, or establish
national completion.

The local recovery protocol includes the origin's initial `/robots.txt` request. Fetcher
`requests` includes both physical dispatches and cache reads; `physical_requests()` excludes cache
hits. Recovery regressions reconcile the server's accepted request targets with that physical
metric, require only the unfinished URL to be reacquired, and forbid physical refetch of completed
pages. Recording-route scenarios apply the same recorded operation twice and require the original
receipt, unchanged facts and unchanged journal payloads on the second application. A separate
live-store route interprets the same cached captures and must retain byte-equal facts, owner
recovery markers and completion payloads after partial acquisition, recovery and completed replay.
These are executable obligations, not a claim that the current checkout has passed them.

## Fixtures

`crates/census-crawl/tests/fixtures/arbiter/` holds the captures, the prototype goldens and the
redacted credential fixtures; `PROVENANCE.json` names the capture hash, URL and byte size of each
and records the prototype-run checks (`NH-arbiter.jsonl` schools and contacts are byte-equal to
the goldens after JSON decode). `tests/fixtures/arbiter/golden_alvirne_coaches.json` is the
prototype's two-row Alvirne result, and `golden_nh_all_coaches_p11.json` is its empty result over
the org-wide page that carries 137 eligible track rows — the divergence the fixtures pin.

The cached-token regression uses a deterministic synthetic entry shell declaring
`index-synthetic-redeploy-20261002.js` and seeds the existing redacted credential slice at that
synthetic URL. This is test construction, not a public capture of that asset. Its requested
URL is known and its historical-style final URL remains unknown. The original bundle capture,
derived slice and `PROVENANCE.json` are unchanged and retain their historical asset URL.
Owned-loopback discovery regressions serve only synthetic HTML and credential-free JavaScript,
exercise redirect/final-URL resolution and a changed declared asset, and assert requested/final
URLs, digests, bytes and immutable archived captures. They do not POST a token or use private
inputs or an external network.

Coding consultation (not census-case advice) used actual local RTX5090
`qwen3.8-27b-uncensored`, response `chatcmpl-2a5f81c0fe4e7233`; raw response:
`var/qwen-5090-evidence-sol-20261002/1790984920420-5c0769e5-5e67-47d2-ae3d-6285e933b516.response.json`.
Its useful suggestions concerned sequential base handling, duplicate-vs-distinct declarations
and truncation coverage. Suggestions to evaluate JavaScript, add browser mocks, change cache
policy/probe source maps, make production authority configurable, accept zero modules, or ignore
the required case/whitespace conventions for module type were rejected. The consultation is
not execution evidence. Main's 2026-10-03 verification exercised 71 Arbiter tests and a fresh
one-school NH provider CLI acquisition: five reported acquisition requests, one school, three
coach rows and zero reported errors. That limited smoke does not qualify the whole organisation
or repair already-settled source failures; exact commands and capture metadata are in
[the evidence ledger](../../../../docs/VERIFICATION-EVIDENCE.md).
