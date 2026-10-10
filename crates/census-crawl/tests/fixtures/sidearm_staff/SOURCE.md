# SIDEARM staff-directory fixture

## Staff capture

- File: `gomats.org__staff-directory__full.html`
- URL: `https://gomats.org/staff-directory`
- Method: GET
- HTTP status: 200
- Content type: `text/html; charset=utf-8`
- Bytes: 305,587
- SHA-256: `8b392547b16ad80dde77bd7941df182804d1c59ee6151308a8da40fdab99b4b8`
- fetched_at: `2026-10-04` (date only; the probe returned no network timestamp)
- Original: `research/sources/coach-coverage-bundle-20261004/probes/sidearm_miramonte/raw/gomats.org__staff-directory__full.html`
- Fixture staged byte-exact by Main; no reserialization, HTML normalization or email rewriting.

## Robots and pacing

- URL: `https://gomats.org/robots.txt`
- HTTP status: 200
- Content type: `text/plain; charset=utf-8`
- Bytes: 6,001
- SHA-256: `454315be725af18052b2009cd7aa02628dbbf5d6445a4818036b110a2ce8b5da`
- fetched_at: `2026-10-04`
- Original: `research/sources/coach-coverage-bundle-20261004/probes/sidearm_miramonte/robots/gomats.org.txt`
The sidearm_staff admission declares gomats.org at 1.0 requests per second, maximum one in-flight request. No separate transport or adapter-specific sleep is introduced.

## Records

The article contains 65 member rows, including sport/level/title cells bound through `headers`, name links, and row-local `firstHalf`/`secondHalf` JavaScript literals. Static recovery joins each row's own trimmed halves with `@`; scripts are not executed. The XC/TF contacts are Brian Henderson (`bhenderson@auhsdschools.org`) and Robert Kennedy (`caljumper259@gmail.com`); the exact Athletic Director title belongs to Sean Hennessy (`shennessy@auhsdschools.org`). Kennedy's separate Flag Football record is retained by the parser but is outside the canonical XC/TF contact scope.

Probe findings and complete provenance: `research/sources/coach-coverage-bundle-20261004/probes/sidearm_miramonte/{FINDINGS.md,manifest.json}`. Prior extraction: `research/sources/coach-coverage-bundle-20261004/extract/SRC-266/`.
