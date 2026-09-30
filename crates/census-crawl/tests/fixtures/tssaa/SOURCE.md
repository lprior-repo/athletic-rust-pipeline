# `tssaa` captures

Byte-exact responses served by `portal.tssaa.org`. The files below are `cp` copies of
`census-prototype/raw/portal.tssaa.org__<sha1-of-url>`, with the corpus's `.meta.json` sidecar
values recorded here; nothing was reformatted or re-serialized.

|Fixture|Corpus file|URL|Status/bytes|sha256|
|---|---|---|---|---|
|`directory_id3.html`|`portal.tssaa.org__7e2c78a9fba681cba03b6ed2`|`https://portal.tssaa.org/common/directory/?id=3`|200, 79,409|`c69f3c2dac4363451b6e7fc23b772bcfa0cd46e468113e79d1edf177d2c71266`|
|`directory_id407.html`|`portal.tssaa.org__07f0c7f079ec6b33f05ce9a3`|`https://portal.tssaa.org/common/directory/?id=407`|200, 47,128|`1f83922a760d68c8634bb4f3236c74e0bbe4e5dfa72296c94644bf1d47b8d0db`|
|`robots.txt`|`portal.tssaa.org__d3f882eba7739af362f02bfb`|`https://portal.tssaa.org/robots.txt`|200, 68|`0f8048a5371ac652f9edff9eea146cd16d76aa9f26fe3a92d5b4c499582719d5`|

## What the captures pin

- `directory_id3.html` — the captured page for school id 3, `<h2>Alcoa High School`; 35
  `class="staffPerson"` table rows, grouped under `card-header` blocks whose text names the sport
  (Baseball, Boys' Basketball, Girls' Basketball, Bowling, Girls' Bowling, Unified Bowling,
  Cheerleading, Boys' and Girls' Cross Country, Football, Boys' and Girls' Golf, ...). Coach email
  addresses are obfuscated as `mail_hide("332779", "lstevenson", 0, "ten.sloohcsaocla")`: the user
  part is literal and the domain argument is spelled backwards (`ten.sloohcsaocla` reverses to
  `alcoaschools.net`). The page does **not** identify its own school id: the only `?id=` anchor is
  `window.location.href="./?id="` with an empty value, so the id is available to the reader only
  through the capture's URL sidecar.
- `directory_id407.html` — school id 407, `<h2>Redemption School of Worship`, and no
  `staffPerson` rows at all. It still carries the school list (below). Its capture proves that a
  school page with no coach rows is a normal page, not a failure.
- `robots.txt` — `User-Agent: *`, `Allow: /common`, `Allow: /access/attendance`, `Disallow: /`.
  The directory under `/common/directory/` is inside the allowed prefix, so these captures sit in an
  allowed path; the file is a fixture because it is a real served response for this host, and no test
  may treat it as a directory page.
- **The school list.** Every school page embeds the same JavaScript literal array — 456 entries of
  the form `{id: '1', name: 'Adamsville High School (Adamsville, TN)'}` — which the table is built
  from at runtime. The array is site-wide data inside a page response, so the reader treats it as the
  address universe for Tennessee and reads entries from whichever capture it is given; the two
  fixtures carry the same 456 entries.

## Corpus shape (measured 2026-09-30, read-only)

460 files match `census-prototype/raw/portal.tssaa.org__[0-9a-f]*`:

- **456 school pages** — 455 carry a `.meta.json` sidecar naming `?id=<n>`; the 456th
  (`portal.tssaa.org__61c3c684588ad668f3d2992f`, 73,070 bytes, one `<h2>` and 16 tables) has no
  sidecar, so its id is not recorded anywhere in the corpus. 430 of the 456 carry at least one
  `staffPerson` row; 26 carry none.
- **3 HTML shells** — `portal.tssaa.org__6cfb724b…` and `portal.tssaa.org__9c683375…` (41,350 bytes
  each) and `portal.tssaa.org__c1170274…` (47,104 bytes): no `<h2>`, no `<table>`, no school name.
  They are not school pages and carry no id.
- **1 robots response** — the 68-byte `robots.txt` above (the file the corpus stored without a
  sidecar, although its content identifies it).

Totals over the 456 school pages: 14,962 `staffPerson` rows, and 405 pages whose cards include a
Track & Field or Cross Country header. The school array appears in every file except the robots
response. No `<tr>` data row and no `?id=<digits>` anchor exists anywhere in the static HTML — the
table is built from the array at runtime — so the array is the only machine-readable school list in a
capture, and a page's own id is available only through its sidecar.
