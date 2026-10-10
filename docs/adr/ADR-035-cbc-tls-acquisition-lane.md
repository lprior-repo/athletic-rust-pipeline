# ADR-035: CBC-compatible acquisition lane for AEAD-incompatible hosts

## Status

Accepted 2026-10-10.

## Context

The static HTTP lane uses rustls, which is AEAD-only. Several athletic association hosts (www.ossaarankings.com, officials.myohsaa.org) complete only TLS 1.2 with CBC cipher suites (ECDHE-RSA-AES256-SHA384) and reset AEAD/TLS 1.3 cipher suites. This results in exit 1 with 'client error (Connect) / Connection reset by peer'.

Browser-lane compatibility has been measured: headless Chromium completes TLS 1.2 with CBC ECDHE-RSA-AES256-SHA (0xC014) against both confirmed hosts with exit 0.

## Decision

Use the browser lane for hosts that require CBC TLS cipher suites. Affected hosts are listed in the source/coverage inventory as transport-requiring-browser. The browser lane preserves pacing, URL admission, raw capture, and error retention.

## Rationale

- The browser lane already handles TLS compatibility issues for other hosts (e.g., athletic.net)
- Headless Chromium supports both AEAD and CBC cipher suites
- The browser lane preserves the required pacing, admission, and capture semantics
- Adding a separate TLS-compatible HTTP lane would require additional infrastructure and maintenance

## Affected hosts

- www.ossaarankings.com (OK tap)
- officials.myohsaa.org (OH tap)

## Evidence

- research/sources/coach-coverage-bundle-20261004/probes/ok-ossaarankings/transport/FINDINGS.md
- research/sources/coach-coverage-bundle-20261004/probes/ok-ossaarankings/transport/browser-lane/FINDINGS.md
- research/sources/coach-coverage-bundle-20261004/probes/oh-ohsaa/transport/browser-lane/FINDINGS.md