# Explicit per-dispatch transport retry ownership

The pinned reqwest 0.13.4 client default permits two extra protocol-NACK retries. The shared
Fetcher client builder previously disabled redirect handling but did not disable that retry
policy. The actual local HTTP/2 REFUSED_STREAM regression observed three request streams
for one reqwest dispatch while application request accounting charged one.

The builder now explicitly selects `reqwest::retry::never()`. Existing destination, origin,
DNS, proxy, redirect, cooldown and source admission policies are unchanged. One Fetcher call
may still perform its bounded admitted redirect sequence; this change is per dispatch and
is not a new limit of one physical request for an entire Fetcher call. The lower transport's
unstarted-connection recovery is a separate mechanism, not a replay of received REFUSED_STREAM.

## Observed red/green evidence

- Red: nextest `c800f5df-3886-40e8-8582-a5c88c51c924`, 2 tests, one passed and one failed,
  1313 skipped, 0.084 test seconds. The real reqwest-default positive control passed with
  streams [1,3,5]. The actual Fetcher regression failed `left=[1,3,5] right=[1]`.
- Green: `d04564e6-2826-4894-90d1-df36deedfc72`, 2/2 passed, 1313 skipped, 0.084 test
  seconds, Moon exit 0. Fetcher produced [1], while the default positive control still
  produced [1,3,5]. Both require the underlying REFUSED_STREAM; Fetcher additionally checks
  exact Transport URL and global/origin physical request accounting.

The real bounded loopback HTTP/2 peer has a 32-frame, 16384-byte/frame bound and a ten-second
joined timeout. No spawned task or external source is involved. This is actual wire request
count evidence, not a copied retry-policy model. It does not qualify durable restart/attempt
identity, other independently constructed clients, or full production source capture lineage.

Focused affected admission regressions and final basic checks will be recorded after execution.

Affected admission lane passed 48/48, 1267 skipped, nextest
`de46bb48-90af-4a30-91ee-d52902f9101a`, 1.757 test seconds, Moon exit 0. It includes
real destination refusal, redirect origin/accounting, cache/304 provenance, authentication-path
refusal, queued 429 admission, shared pacing/cooldown and both HTTP/2 retry regressions.
Independent changed-source review found no blocking code defect and pinned both source files;
its broader acceptance remained blocked pending the subsequently executed gates.


The first basic run passed all-target compilation and strict production source lint, then
format check failed solely on the new test helper's multi-line signature. A combined
fmt-write/fmt attempt ran the check first and is not passing evidence. The owner subsequently
runs the write and check as separate ordered Moon commands; no source gate is weakened.

Ordered Moon formatting write and subsequent check both passed, exit 0. The sole change was
collapsing the new test helper signature; production retry logic and test assertions were
unchanged. Final typed-parser integration will receive its own combined checks and pins.

## Optional geocoding transport

The separately reached Google/USPS transport also falls under ADR-002's automatic retry-owner
contract. Its shared builder now explicitly disables protocol-NACK retries. The bounded wire
fixture is shared with Fetcher tests rather than duplicated. Model/ingress clients were
inspected but unchanged: no currently reachable HTTP/2 retry defect was established for the
plain-HTTP loopback model endpoint; ingress safe-NACK retry is not evidence of duplicate
accepted Restate/external effects. Geocode's existing redirect behavior remains unchanged,
so this repair is per dispatch, not a guarantee of one request across a redirect chain.

Geocode red nextest `3433c6a5-610b-474e-869b-b41486c27ba0`: three tests, two passed,
one failed, 1332 skipped, 0.507 seconds. Geocode observed [1,3,5] versus expected [1];
existing default positive control and repaired Fetcher passed.
Green plus geocode regressions: `748d26b2-af0d-4fbf-9f84-9d269af560ef`, 21/21 passed,
1314 skipped, 10.115 test seconds, Moon exit 0. Includes real HTTP transport, timeout and
error redaction plus all three protocol fixtures. Credential values in fixtures are synthetic;
no actual vendor credentials or external APIs were used.
