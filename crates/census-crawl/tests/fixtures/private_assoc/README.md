# `private_assoc` fixtures — empty by evidence

This directory holds **no captures**. The reader's three hosts (`www.nais.org`, `www.cape-ed.org`,
`www.nassp.org`) have no capture and no robots verdict in either research tree: every occurrence of
the host names under `census-prototype/` and under this worktree is another school's page linking the
association, and every `robots*` file in the trees belongs to a different host. The module README
records the exact paths searched and the lines that were found.

This directory carries only this README, which is the contract file `cargo xtask replay` skips:
`cargo xtask source-fixture private_assoc` lists that one file, and `cargo xtask replay private_assoc`
refuses with `no captures under …: the directory holds no body to replay`. That refusal is the finding,
not a defect.

Nothing may be added here by hand: contract §3 admits byte-exact captures only, and contract §2 puts
hand-made inputs in the reader's own `tests.rs`, not in this directory. An invented body would make
the parser look capture-tested when it is not.

## What a first real capture must satisfy

- One file per distinct response shape the reader reads (a listing page, and whatever pagination
  shape the site uses).
- Bytes exactly as served (`.html`), not prettified, re-serialized or excerpted.
- Name it `<surface>_<which>.html`, lowercase with underscores, e.g. `listing_nais.html`.
- Add `SOURCE.md` next to it with the request URL, the capture date, the robots status and the
  sha256 — the three hosts currently have no robots verdict, so the fetch that produces the first
  capture has to record one.
- Then re-check the selectors in the module README against the capture and correct the README and
  the reader together.

## Provenance

None yet. The first capture's `SOURCE.md` is what makes the fixture corpus auditable; without it the
directory stays documentation-only and the reader stays marked unverified against a live response.
