# Max-envelope test clock race

Test-only delta, no production edits or execution. Existing Store Clock exposes today_iso8601, whose SystemClock implementation uses UTC RFC3339 SecondsFormat::Secs. acquisition::codec::stage hardcodes SystemClock and has no injected clock seam. Two separate calls can differ at any second boundary.

The tests now measure encoded size using a fixed20-byte RFC3339-seconds specimen. The admitted actual row is checked for exact65,536 bytes and exact decoded key/payload; its own retained at field is used to reconstruct the exact canonical envelope. This avoids comparing separate real-clock samples and still checks exact bytes. No sleep, retry, relaxed budget or false clock-injection claim is introduced.
