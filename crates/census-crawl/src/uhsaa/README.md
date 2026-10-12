# UHSAA school-directory projection

The public Utah member directory at `https://uhsaa.org/school-directory-new/` publishes school
names and numeric `schoolID` links. School profile captures publish postal fields and TF/XC coach
listings. Retained public documents live under `tests/fixtures/uhsaa/`; their acquisition inventory
is recorded in that directory's `SOURCE.md`.

## Ownership and capture qualification

`map_capture` receives a published directory link and a retained `FetchOutcome`. The indexed ID,
requested capture URL and any observed final URL must agree; the live profile heading must match
the indexed school. The digest is checked against retained bytes, and evidence uses the capture's
bounded RFC3339 `fetched_at`, never the caller's evaluation date.

Only a live school-details postal item marked by the published map-marker field becomes a typed
`SchoolPostalAddress`. Its association-school owner, capture URL, full SHA256 and original retrieval
time remain attached. The published city participates in canonical school identity. Inert markup,
prose and raw evidence notes do not supply typed postal ownership. Missing address means unknown;
malformed locality, foreign jurisdiction or conflicting published fields refuse projection.

`parse_school_list` is the admitted `uhsaa` reader for `--association-directory uhsaa=PATH`.
It reads the raw directory document into Utah state-record identities and names, not profile postal
claims. A profile document without indexed school-owner links is refused. Directory entries have
no invented address; the standard lane binds their digest and supplied source provenance.

## Appointment qualification

The recorded profiles do not publish an appointment school year. Every retained TF/XC listing has
an explicit `CoachTenure::Unknown` fact binding its role/program/mailbox to the same capture. A
retrieval date, general site date or seasonless listing never establishes current tenure.
Collection preserves these listings and typed school addresses but records an unfinished,
blocked appointment-season qualification. No active-season contact is certified. A future
published appointment-year format requires explicit source qualification before current tenure
can be emitted.

`uhsaa_projection_v3` separates this owner-qualified postal/unknown-tenure projection from older
unqualified receipts. Identical capture replay does not advance freshness or reappend entities.

## Focused acceptance

Run through the repository Moon launcher:

```sh
env -u CI tools/moon-local run pipeline:tests -- -E 'test(uhsaa) | test(school_address)'
```

The retained-capture collector regression exercises real offline Fetcher/Store readback and replay.
The service smoke publishes and independently reopens the directory generation, runs the school
join, retains the owned profile postal claim and confirms unknown appointment tenure. Replay test
metadata is deliberately injected; it is not evidence of the original capture's time of day or a
fresh network acquisition.
