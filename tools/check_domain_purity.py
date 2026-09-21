#!/usr/bin/env python3
"""Domain-crate purity proof: the `census-domain` dependency tree carries no async runtime, store
engine, HTTP client, service framework or browser engine.

This is the enforceable form of the Phase 3 acceptance criterion ("ban proof that the domain
crate's tree has zero async/I/O deps"). cargo-deny's `wrappers` bans express the inverse relation
(a banned crate allowed only under a listed wrapper), so the tree itself is the evidence here:
normal edges only, which excludes dev-dependencies and build scripts, so test-only crates cannot
taint the verdict either way.
"""

from __future__ import annotations

import subprocess
import sys

BANNED = frozenset(
    {
        "tokio",
        "fjall",
        "reqwest",
        "serde_json",
        "restate-sdk",
        "restate-sdk-shared-core",
        "chromiumoxide",
        "chromiumoxide-cdp",
        "chromiumoxide_types",
        "hyper",
        "hyper-util",
        "axum",
        "axum-core",
        "tower",
        "tower-http",
        "mio",
        "h2",
        "rustls",
        "openssl",
        "socket2",
        "tungstenite",
        "async-tungstenite",
        "tokio-util",
        "tokio-rustls",
    }
)


def main() -> int:
    result = subprocess.run(
        ["cargo", "tree", "-p", "census-domain", "--edges", "normal", "--prefix", "none"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(result.stderr.strip())
        print("domain purity: could not resolve the census-domain tree", file=sys.stderr)
        return 1
    packages = sorted({line.split()[0] for line in result.stdout.splitlines() if line.strip()})
    print("  census-domain normal tree: " + ", ".join(packages))
    violations = [name for name in packages if name in BANNED]
    if violations:
        print("  FORBIDDEN dependencies present: " + ", ".join(violations), file=sys.stderr)
        return 1
    print("  no async/I-O dependency present")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
