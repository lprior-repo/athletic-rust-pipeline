#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
SCRATCH_ROOT="${SCRATCH_STORE:-${TMPDIR:-/tmp}}"

for tool in mktemp ls python3 sha256sum awk wc tr; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'SKIPPED: %s not available\n' "$tool"
        exit 0
    fi
done

resolve_probe() {
    local evidence="$1"
    local cargo_bin="${CARGO:-cargo}"
    if ! (cd "$REPO_ROOT" && "$cargo_bin" build --example enospc -p census-store --message-format=json) > "$evidence/build.json" 2> "$evidence/build.err"; then
        return 1
    fi
    python3 - "$evidence/build.json" <<'PY'
import json
import sys

for line in open(sys.argv[1], encoding="utf-8"):
    try:
        message = json.loads(line)
    except ValueError:
        continue
    if message.get("reason") != "compiler-artifact":
        continue
    target = message.get("target") or {}
    if target.get("name") != "enospc" or "example" not in (target.get("kind") or []):
        continue
    executable = message.get("executable")
    if isinstance(executable, str) and executable:
        print(executable)
        break
    for name in message.get("filenames") or []:
        if isinstance(name, str) and name:
            print(name)
            break
    break
PY
}

probe_target_dir() {
    local target_dir="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
    if [[ "$target_dir" != /* ]]; then
        target_dir="$REPO_ROOT/$target_dir"
    fi
    printf '%s\n' "$target_dir"
}

select_probe() {
    local evidence="$1"
    local notes="$evidence/selection.err"
    local cargo_bin="${CARGO:-cargo}"
    local selected
    if ! selected="$(resolve_probe "$evidence")"; then
        printf 'the build reported no usable cargo artifact\n' >> "$notes"
        return 1
    fi
    if [[ -z "$selected" ]]; then
        printf 'the build reported no enospc example artifact\n' >> "$notes"
        return 1
    fi
    if [[ ! -x "$selected" ]]; then
        printf 'the reported artifact %s is not executable\n' "$selected" >> "$notes"
        return 1
    fi
    local target_dir
    target_dir="$(probe_target_dir)"
    case "$selected" in
        "$target_dir"/*) ;;
        *)
            printf 'the selected probe %s is outside the Cargo target directory %s\n' "$selected" "$target_dir" >> "$notes"
            return 1
            ;;
    esac
    printf '%s  %s\n' "$(sha256sum "$selected" | awk '{print $1}')" "$selected" > "$evidence/probe-identity.txt"
    {
        printf 'bytes %s\n' "$(wc -c < "$selected" | tr -d ' ')"
        printf 'cargo %s\n' "$("$cargo_bin" --version 2>/dev/null || printf 'unknown')"
        printf 'source_revision %s\n' "$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || printf 'unknown')"
        printf 'target_dir %s\n' "$target_dir"
    } >> "$evidence/probe-identity.txt"
    printf '%s\n' "$selected"
}

routing_check() {
    local root
    root="$(mktemp -d "$SCRATCH_ROOT/enospc-09-routing-XXXXXX")"
    mkdir -p -- "$root/moon-target/debug/examples" "$root/stale-repo/target/debug/examples" "$root/clean-repo" \
        "$root/evidence-moon" "$root/evidence-clean" "$root/evidence-absent"
    printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$0" >> "%s"\n' "$root/built.marker" > "$root/moon-target/debug/examples/enospc"
    printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$0" >> "%s"\n' "$root/sentinel.marker" > "$root/stale-repo/target/debug/examples/enospc"
    chmod +x "$root/moon-target/debug/examples/enospc" "$root/stale-repo/target/debug/examples/enospc"
    printf '#!/usr/bin/env bash\ncat "$STUB_CARGO_PLAN"\n' > "$root/stub-cargo"
    chmod +x "$root/stub-cargo"
    local built="$root/moon-target/debug/examples/enospc"
    local sentinel="$root/stale-repo/target/debug/examples/enospc"
    local plan="$root/plan.jsonl"
    local selected
    printf '{"reason":"compiler-artifact","target":{"kind":["example"],"name":"enospc"},"executable":"%s"}\n' "$built" > "$plan"
    if ! selected="$(
        export REPO_ROOT="$root/stale-repo"
        export CARGO="$root/stub-cargo"
        export STUB_CARGO_PLAN="$plan"
        export CARGO_TARGET_DIR="$root/moon-target"
        select_probe "$root/evidence-moon"
    )"; then
        printf 'FAIL: the routing check could not select the built probe (%s); evidence retained %s\n' "$(cat "$root/evidence-moon/selection.err" 2>/dev/null)" "$root"
        return 1
    fi
    if [[ "$selected" != "$built" ]]; then
        printf 'FAIL: the routing check selected %s, not the freshly built %s; evidence retained %s\n' "$selected" "$built" "$root"
        return 1
    fi
    if [[ "$(head -1 "$root/evidence-moon/probe-identity.txt" | awk '{print $1}')" != "$(sha256sum "$built" | awk '{print $1}')" ]]; then
        printf 'FAIL: the recorded probe hash does not bind the built artifact; evidence retained %s\n' "$root"
        return 1
    fi
    if ! "$built"; then
        printf 'FAIL: the built fixture probe did not run; evidence retained %s\n' "$root"
        return 1
    fi
    if [[ "$(cat "$root/built.marker" 2>/dev/null)" != "$built" ]]; then
        printf 'FAIL: the executed probe was not the built artifact; evidence retained %s\n' "$root"
        return 1
    fi
    if [[ -e "$root/sentinel.marker" ]]; then
        printf 'FAIL: the stale default-target sentinel ran; evidence retained %s\n' "$root"
        return 1
    fi
    if ! selected="$(
        export REPO_ROOT="$root/clean-repo"
        export CARGO="$root/stub-cargo"
        export STUB_CARGO_PLAN="$plan"
        export CARGO_TARGET_DIR="$root/moon-target"
        select_probe "$root/evidence-clean"
    )"; then
        printf 'FAIL: the routing check required a default target directory (%s); evidence retained %s\n' "$(cat "$root/evidence-clean/selection.err" 2>/dev/null)" "$root"
        return 1
    fi
    if [[ "$selected" != "$built" ]] || [[ -e "$root/clean-repo/target" ]]; then
        printf 'FAIL: the routing check fell back to a default target directory; evidence retained %s\n' "$root"
        return 1
    fi
    printf '{"reason":"compiler-artifact","target":{"kind":["lib"],"name":"census-store"},"filenames":[]}\n' > "$plan"
    if selected="$(
        export REPO_ROOT="$root/stale-repo"
        export CARGO="$root/stub-cargo"
        export STUB_CARGO_PLAN="$plan"
        export CARGO_TARGET_DIR="$root/moon-target"
        select_probe "$root/evidence-absent"
    )"; then
        printf 'FAIL: the routing check accepted an absent artifact %s; evidence retained %s\n' "$selected" "$root"
        return 1
    fi
    if [[ -e "$root/sentinel.marker" ]]; then
        printf 'FAIL: the stale default-target sentinel ran; evidence retained %s\n' "$root"
        return 1
    fi
    printf 'ROUTING: the selected probe was the built artifact under the selected target directory; the stale %s never ran\n' "$sentinel"
    return 0
}

routing_check || exit 1

for tool in unshare mount findmnt timeout cp; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'SKIPPED: %s not available\n' "$tool"
        exit 0
    fi
done
if ! unshare --user --map-root-user --mount --propagation private true 2>/dev/null; then
    echo "SKIPPED: unprivileged user+mount namespace not supported"
    exit 0
fi

mkdir -p -- "$SCRATCH_ROOT"
SCRATCH_DIR="$(mktemp -d "$SCRATCH_ROOT/enospc-09-XXXXXX")"
printf 'EVIDENCE: %s\n' "$SCRATCH_DIR"

if ! PROBE_BIN="$(select_probe "$SCRATCH_DIR")"; then
    printf 'FAIL: the probe artifact could not be resolved from the build; evidence retained %s\n' "$SCRATCH_DIR"
    cat "$SCRATCH_DIR/selection.err" 2>/dev/null || true
    cat "$SCRATCH_DIR/build.err" 2>/dev/null || true
    exit 1
fi
PROBE_SHA="$(sha256sum "$PROBE_BIN" | awk '{print $1}')"
printf 'PROBE-ID: %s\n' "$(awk '{printf "%s;", $0}' "$SCRATCH_DIR/probe-identity.txt")"

RC=0
timeout --signal=TERM 300s unshare --user --map-root-user --mount --propagation private -- \
    bash -c '
        set -uo pipefail
        artifact=$1
        probe=$2
        probe_sha=$3
        mountpoint="$artifact/tmpfs"
        store="$mountpoint/store"
        probe_pid=""
        preserve() {
            local code=$?
            trap - EXIT INT TERM
            if [[ -n "$probe_pid" ]]; then
                if kill -0 "$probe_pid" 2>/dev/null; then
                    if ! kill -TERM "$probe_pid"; then
                        echo "FAIL: owned probe termination failed"
                        code=1
                    fi
                fi
                local stopped=0
                wait "$probe_pid" || stopped=$?
                printf "CLEANUP: owned probe reaped with exit %s\n" "$stopped"
            fi
            if [[ -d "$store" ]]; then
                if ! cp -a -- "$store" "$artifact/preserved-store"; then
                    echo "FAIL: cold fault-store preservation failed"
                    code=1
                fi
            fi
            printf "CLEANUP: namespace exit %s; artifact retained %s\n" "$code" "$artifact"
            exit "$code"
        }
        trap preserve EXIT
        trap "exit 130" INT
        trap "exit 143" TERM
        mkdir -- "$mountpoint" || exit 1
        if ! mount -t tmpfs -o size=64M tmpfs "$mountpoint"; then
            echo "SKIPPED: mount tmpfs not supported inside unprivileged namespace"
            exit 0
        fi
        if ! printf "%s  %s\n" "$probe_sha" "$probe" | sha256sum --check --status; then
            echo "FAIL: the probe artifact hash does not bind the binary about to run"
            exit 1
        fi
        "$probe" "$store" &
        probe_pid=$!
        result=0
        wait "$probe_pid" || result=$?
        probe_pid=""
        exit "$result"
    ' _ "$SCRATCH_DIR" "$PROBE_BIN" "$PROBE_SHA" > "$SCRATCH_DIR/probe.out" 2>&1 || RC=$?

SKIPPED=false
PASS_COUNT=0
PASS_KIND=""
ENOSPC_COUNT=0
CLEANUP_SEEN=false
while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
        SKIPPED:*)
            SKIPPED=true
            printf '%s\n' "$line"
            ;;
        'PASS: kernel ENOSPC refused '*)
            PASS_COUNT=$((PASS_COUNT + 1))
            PASS_KIND="kernel"
            printf 'PROBE-%s\n' "$line"
            ;;
        'PASS:'*)
            PASS_COUNT=$((PASS_COUNT + 1))
            PASS_KIND="unexpected"
            printf 'PROBE-%s\n' "$line"
            ;;
        'ENOSPC: atomic commit refused '*)
            ENOSPC_COUNT=$((ENOSPC_COUNT + 1))
            printf '%s\n' "$line"
            ;;
        'CLEANUP: namespace exit 0; artifact retained '*)
            CLEANUP_SEEN=true
            printf '%s\n' "$line"
            ;;
        *)
            printf '%s\n' "$line"
            ;;
    esac
done < "$SCRATCH_DIR/probe.out"

if [[ "$RC" -ne 0 ]]; then
    printf 'FAIL: owned probe/namespace exited %s; evidence retained %s\n' "$RC" "$SCRATCH_DIR"
    exit 1
fi
if [[ "$SKIPPED" == true ]]; then
    exit 0
fi
if [[ "$PASS_COUNT" -ne 1 || "$PASS_KIND" != "kernel" ]]; then
    printf 'FAIL: expected exactly one kernel-ENOSPC probe PASS line, observed %s (%s); evidence retained %s\n' \
        "$PASS_COUNT" "${PASS_KIND:-none}" "$SCRATCH_DIR"
    exit 1
fi
if [[ "$ENOSPC_COUNT" -ne 1 ]]; then
    printf 'FAIL: expected exactly one reached kernel ENOSPC refusal, observed %s; evidence retained %s\n' \
        "$ENOSPC_COUNT" "$SCRATCH_DIR"
    exit 1
fi
if [[ "$CLEANUP_SEEN" != true ]]; then
    printf 'FAIL: fault namespace did not report a clean tmpfs exit; evidence retained %s\n' "$SCRATCH_DIR"
    exit 1
fi
if [[ ! -d "$SCRATCH_DIR/preserved-store/fjall" ]] || [[ -z "$(ls -A -- "$SCRATCH_DIR/preserved-store" 2>/dev/null)" ]]; then
    printf 'FAIL: cold fault-store preservation is missing or empty; evidence retained %s\n' "$SCRATCH_DIR"
    exit 1
fi
if [[ "$(sha256sum "$PROBE_BIN" | awk '{print $1}')" != "$PROBE_SHA" ]]; then
    printf 'FAIL: the executed probe artifact changed on disk; evidence retained %s\n' "$SCRATCH_DIR"
    exit 1
fi
echo "PASS: bounded kernel ENOSPC refused an atomic commit; exact acknowledged receipts, cold readback, unchanged replay and a new atomic recovery write verified across reopens"
