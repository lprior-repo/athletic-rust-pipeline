# Reconciled checksums and reference repair - 2026-10-05

Resolves the 13 mismatches and 1 missing reference reported by
`audit/tap-tree-rehash-20261005.md` (82 trees, 394 claimed file/digest pairs: 380 OK, 13 mismatch,
1 missing; no raw-response digest mismatched). This document is the external reconciliation record:
no published digest was re-pinned in place, no capture was rewritten, and the audit's own table
stays as the historical claim.

## Dispositions

| Path (bundle-relative) | Published in sweep | Current authoritative | Cause class |
|---|---|---|---|
| extract/SRC-054/REPORT.md | `3ad4fe8b7f7ca4553aaf104a9415a327c0428c5672c9b80d640b7b92fdc35bbd` | `e604900e6ef1ab55e6d112251fd15f755cf820e2ebb851dc64f2c6dd976e11fc` | report revised after sweep |
| extract/SRC-054/sweep.json | `183295ac3ec93d31d8bc73509c506a6053ffe1fae1b2d4c42109931bed926bd7` | `08e40f03741688999e00eb472972d2d596f0826268b0acc57debddb2bac8907c` | self-referential entry |
| extract/SRC-068/REPORT.md | `34a1674e6d2e9663f56933108c7a9566eb4dc80b403a9a506f71be56fee3187c` | `c69aab3c20c686fcd8c533fc220a2221ba9992ee5abcb8c3645bb585b143ba35` | report revised after sweep |
| extract/SRC-068/sweep.json | `a9332ad5cb52fa5977d50f873a9022bb37a14657f97da6e86088f842a8fffd20` | `f801b14b429ba087203578638e584adbe6691ed806add04364eb44874d79dd6d` | self-referential entry |
| extract/SRC-100/REPORT.md | `84127ad606957fd353a1f0c314958bcfa3075a65b7f09ee4c91b3fb6f735bb79` | `6facb05ce3578e8dcd3892ed3ef552440e1e43295fd82b77d652ca19f5d60152` | report revised after sweep |
| extract/SRC-100/excluded_rows.csv | `cc6db861a2514f2610184ff711c000d224154b00b813447013afc7e76df961e5` | `ec23e97a43aff11452c189d5523d5a4dffbe02373b42964bdb72105fcde6a9ea` | superseded derived data |
| extract/SRC-100/rows.csv | `2e71e6ea58a5d9ad40b748cfb47931c22a18d177844105a0958c0e77bb0c275b` | `e3bd01a0fb5fb19106b53205fe9dc9ec12ff51791830a2f77bafe087ee0403d9` | superseded derived data |
| extract/SRC-175/REPORT.md | `427578a59d0b577e7147f76d0ee6c5a3a1f9fafeebbef1cf665d24c473f86507` | `a3ab9f80b6dbe53ab2d3046558072be5724b54155d6c7356bcffc24a6c973ad4` | report revised after sweep |
| extract/SRC-175/sweep.json | `1bf6cc245bf3d5a70fc95cc191740f42a7695685f9496426f534bd045e7db5ef` | `f384dc2c45f280b24ff6595ee070b914ffcf215dd59c2bb214ca80a81d9a0256` | self-referential entry |
| extract/SRC-222/REPORT.md | `d901ebfeaf7bcb84fdc6b295dd238355510f261bf68982948f6b8beb6a6e1eca` | `7f1c8ebe0f8b2b5d56061aea3f9c6b5abfa147d85b73413d1d835cacf5785c01` | report revised after sweep |
| extract/SRC-222/sweep.json | `dda4b6bb5612ae8774a37f3d8a9daf735d8ab71b6b20c784e1b4716b6eedf396` | `ee957bf1e44742963595f016fb40c4d1d44e21743e749ad622cb35e24d77b530` | self-referential entry |
| probes/sources/SRC-106/sweep.json | `976acef3311b4f9b91f8937edf76f6d847aed55b08b68f12417944c020dae6aa` | `2bd2abb9dcccc4ed1d63a43069899a56172e6a4e10345a4694ed15c299dc1b0e` | self-referential entry |
| probes/sources/SRC-114/sweep.json | `e2bb7b2e1ed768fe545c4b122152a1ac496b705481278492cdd8ade41cbff45c` | `71083f7a1b5feea41cc352b6455aa40fb315d6c89ce87f06576f1463f0709dd2` | self-referential entry |

## Cause classes

- **Report revised after sweep (5).** `extract/SRC-054`, `/068`, `/175`: the sweep was written at
  14:55:27 local and the reports were finished 17-21 seconds later
  (`stat -c '%y %n'` on the three pairs), the same ordering pattern as `SRC-100` and `SRC-222`. The
  current `REPORT.md` bytes remain the authoritative documentation; for `SRC-100` the current
  `rows.csv`/`excluded_rows.csv` digests are explicitly published in its `Main integration` section.
- **Self-referential entry (6).** Each `sweep.json` records a digest of the checksum document
  itself. Such an entry cannot match once the document is written, and re-pinning it would not
  create a stable self-hash. Policy: self-entries are excluded; the external record is this
  document. Future sweeps must omit self-entries or be hashed externally.
- **Superseded derived data (2).** `SRC-100/rows.csv` and `excluded_rows.csv` were revised after the
  sweep when the 2,154-row transform split into 2,141 importable plus 13 excluded rows (3 empty
  `school`, 10 `NOT IN OHIO`). The current digests and the import readback (rows=2,141, schools=630,
  rejections=0) are recorded in `SRC-100/REPORT.md`; the sweep's values describe the pre-split file.

## SRC-133 reference repair

`probes/sources/SRC-133/manifest.json` named `../iatrackcoaches/robots/www.iatrackcoaches.org.txt`,
which resolves one level short (under `probes/sources/`). Corrected to
`../../iatrackcoaches/robots/www.iatrackcoaches.org.txt`. The target exists and hashes to the
published `42c1b29b65ce022d1e3bedb325c961d22de02b487a6a7272b7624cbe744fd253`
(sha256sum re-run 2026-10-05), so only the reference was wrong, not the claim.

## Limits

Recomputation here was limited to `sha256sum` and `stat` over the named paths; no capture was
rewritten, no store opened, no network request issued, and no historical CLI re-executed. The
audit's per-tree structural findings and its 108 undigested files are not addressed here (tracked
separately as `athletic-rust-pipeline-fwzp`).
