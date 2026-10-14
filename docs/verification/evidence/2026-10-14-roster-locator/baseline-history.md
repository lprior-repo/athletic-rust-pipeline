# Baseline compatibility oracle history

This evidence is additive. It preserves both old-production probes and does not regenerate a compatibility golden from changed production code.

## V1: executed, weaker than intended

The test-only V1 probe was applied and executed by Root against old production commit `3708d6bab0b08c390887c562a89e8c6714255a5d`, then removed before implementation. Its raw log reports one passing test, 3,271 skipped, Nextest run `684a5b5e-e3d1-4b36-bb16-7282385a4467`. Root separately attests outer exit 0; that is an attributed execution receipt, not a raw tool export.

The unchanged output is `inputs/baseline-probe/legacy-bytes.json`: 4,001 bytes, SHA-256 `0114d5deea680ddcfa56baec2fa5487f8f6fd594034dc0f190722ae7ad57f9cc`. Its canonical tuple contains two athletes but zero teams. The fixture's former season markup did not match production season parsing, making this weaker than the required nonempty-family compatibility example. It is retained as history and is not the installed golden.

## V2: executed with stronger independent guards

The separate test-only V2 probe used the production-recognized data-point/icon icon-yes markup and asserted two athletes, two teams, two athlete observations, exact gender/cohort order, one MissingGraduationYear rejection, Partial state, no unfinished row, and exact counts before exporting. Root executed and removed this patch against the same old production code before applying the located-roster change.

Its raw log reports one passing test, 3,271 skipped, Nextest run `445729b4-0a11-472d-896d-8054ba1bb501`. Root separately attests outer exit 0. The unchanged raw output is `inputs/baseline-probe-v2/legacy-bytes.json`: 4,971 bytes, SHA-256 `5dce866cca6bab85495c5dc46d9e9e6610d7eda53e63305374153b6e88827fed`.

The installed `legacy_roster_bytes.json` is exactly these V2 bytes plus one terminal LF outside the serialized payload strings: 4,972 bytes, SHA-256 `7dcb2f41bc3c18d1a4a658674e795cce7252c0d2c10cc3e1d07d3238523d6848`. Parsed JSON values and every embedded canonical/Journal/digest string are identical. The two byte representations must retain separate hashes.

## What this establishes

The candidate compatibility test compares exact canonical tuple JSON, Journal JSON, fact digest and effect digest against the old-code V2 output. It does not merely compare two changed implementations sharing a serializer. This is still one finite old-code compatibility case, not an independent oracle for every recruiting semantic. Its `unfinished: null` is not execution coverage of Some(locator) Journal serialization. The capacity test separately exercises an unfinished parser locator.

`baseline-provenance.json` records measured lengths, hashes and JSON relationships. `source-origin.json` confirms all 18 retained existing source preimages equal Git bytes at the old production commit and at the documentation-only successor HEAD. The four added files have no baseline blob. The probe patches, base/proposal sources and logs remain separate from the 22-file implementation patch. Numeric outer exits are explicitly attributed to Root's `root-command-receipts.json`, not inferred from PASS text.
