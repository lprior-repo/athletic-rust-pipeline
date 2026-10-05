# Home Campus Fixture Sources

Capture date: 2026-10-04. User-Agent:
`athletic-rust-pipeline-research/0.1 (+https://github.com/lprior-repo/athletic-rust-pipeline)`.
Every file is the response body exactly as served (a plain `GET`; the details JSON carries the
directory's own XHR context), and the bytes and sha256 below were measured on the committed files.

## robots.txt

`https://www.cifsshome.org/robots.txt` — 200, 24 bytes, sha256
`e5c4b84484ee4216e9373be99380320c25dd94805f99f0a805846f087636553f`. The whole body is
`User-agent: *\nDisallow:` — nothing disallowed. `www.cifnshome.org` and `www.fhsaahome.org` serve
the identical 24 bytes; copies of all three sit under
`research/sources/coach-coverage-bundle-20261004/probes/home_campus/robots/`.

## Directory Sections

`widget/school/directory?section=<n>` — one
`<button class="… school-btn …" id="school-button-<id>" data-id="<id>">` per school; the button
body is the published school name (`(City)` suffix; HTML entities such as `&#039;` as served).

- `directory_section_10.html` — 200, 230,370 bytes, sha256
  `f79590dd8f18e376b9a93ee98c462319b035cadac5bf6bf79c8b5ed04daf103b` — FHSAA (FL), 880 buttons.
  First: `Abundant Life Christian (Margate)` (id 2397); second: `Academy at the Lakes (Land
  O'Lakes)` (id 2373).
- `directory_section_12.html` — 200, 133,880 bytes, sha256
  `3c9a5d71bc72f2ebca6449cbb3e4c705867028dc0b4d9d19904a2ed5419413f2` — NJSIAA (NJ), 452 buttons,
  every name non-empty.

## School Details

`widget/get-school-details/<id>/details` answers `application/json` only with the directory page's
own XHR context (`X-Requested-With: XMLHttpRequest` plus a same-origin `Referer`); without them the
same URL answers 403. Shape: `school`, `coaches[]` (`firstname`, `lastname`, `sport`, `sport_id`,
`level_name`, `aft_name`, `email`), `athleticFaculties[]` (`firstname`, `lastname`, `aft_name`,
`email`, `work_phone`), plus `grades` and `geoGroups`.

- `details_19.json` — Arcadia (CA), 7,813 bytes, sha256
  `0667c404b66f24f6395eb632bde227772683ed38d29ca36ed788f52772631eee` — 24 coaches (four XC/TF head
  coaches; one `user_id` covers both XC and TF), 6 faculty rows, one Athletic Director (Levi Sieg,
  `lsieg@ausd.net`), and one coach row whose `aft_name` is null.
- `details_1872.json` — Bolles (Jacksonville, FL), 8,062 bytes, sha256
  `5dafbbeac5a450dd065dfd77cf2485658b513e6c8bcca8005150f795ca4d9dd4` — 26 coaches including one
  all-null `Track & Field, Girls` row, 8 faculty rows, one Athletic Director (Rock Pillsbury,
  `pillsburyr@bolles.org`).
- `details_3374.json` — Abraham Clark (NJ), 958 bytes, sha256
  `6c63554f9f89ccb1af1c30369e987295da327e3f187d3cbdc31a64dacbc7d9b6` — no coaches and no faculty
  rows; only the school record is published.

`.md` files here are documentation, not test input.
