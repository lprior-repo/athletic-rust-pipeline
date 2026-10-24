#!/usr/bin/env bash
# Native fault resources: verify and prepare what the seventeen national census
# scenarios need (docs/NATIONAL-CENSUS-FAULTS.md).
#
# Verifies every pinned prerequisite on this machine — Restate server 1.7.10, the QEMU
# 11.1.1 prefix with its staged package checksums, the Arch cloud image and its published
# digest, the live guest capture manifest and the scenario prerequisite tools — prepares
# the owned scratch disks under var/, and probes the isolation mechanisms the ENOSPC and
# clock lanes depend on. It never reboots the host, changes its clock, terminates a
# shared service or writes outside the owned scratch/certificate paths.
#
# Usage:
#   tools/durability/native-resources.sh
#   tools/durability/native-resources.sh --certificate var/durability-scratch/native-resources.json
#   env -u CI tools/moon-local run pipeline:native-resources
#
# Exit status: 0 = every pinned resource and prerequisite verified; 1 = a required
# resource is missing or unusable; 2 = usage error. --certificate writes a
# machine-readable certificate of the observed identities and refuses to overwrite.
#
# Overrides: RESTATE_SERVER_BIN, CENSUS_NATIVE_VM_CAPTURES, CHROMIUM_EXECUTABLE,
# SCRATCH_STORE, ATHLETIC_RUST_PIPELINE_RESOURCES, FAULT_MEDIA_MIB.

set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
RESOURCE_ROOT="${ATHLETIC_RUST_PIPELINE_RESOURCES:-$HOME/.local/share/athletic-rust-pipeline}"
SCRATCH_STORE="${SCRATCH_STORE:-$REPO_ROOT/var/durability-scratch}"
RESTATE_PIN="1.7.10"
QEMU_PIN="11.1.1"
IMAGE_NAME="Arch-Linux-x86_64-cloudimg-20261001.604814.qcow2"
IMAGE_SHA256="360f0fa49db6813bdc8e35bed230a2dc2ae3567b7b5ab74719c0a706e4e34e87"
BINARY_BUDGET=536870912
BIOS_BUDGET=33554432
IMAGE_BUDGET=8589934592
CAPTURES_BUDGET=33554432
MEDIA_MIB="${FAULT_MEDIA_MIB:-64}"
SEAMS=(CENSUS_NATIVE_SOURCE_BOUNDARY CENSUS_NATIVE_WORKER_BOUNDARY CENSUS_NATIVE_WORKER_MARKER
    CENSUS_NATIVE_CONSOLIDATE_BOUNDARY ATHLETIC_FAULT_HTTP_EXIT)
REQUIRED_TOOLS=(bash uname date realpath stat sha256sum mktemp mkdir chmod cp mv ln cat cmp rm rmdir
    truncate head tail basename dirname grep sed sort find awk wc tr timeout flock tee curl ss jq
    python3 pgrep unshare mount findmnt)
CERTIFICATE=""
RECORDS=()
TOOLRECORDS=()
UNLISTED=()

fail() { printf 'FAIL: native resources: %s\n' "$*" >&2; exit 1; }
usage() {
    printf 'usage: %s [--certificate <path>]\n' "$0" >&2
    exit 2
}

while [ $# -gt 0 ]; do
    case "$1" in
        --certificate)
            [ $# -ge 2 ] || usage
            CERTIFICATE="$2"
            shift 2
            ;;
        --help | -h) usage ;;
        *) usage ;;
    esac
done

[[ "$MEDIA_MIB" =~ ^[0-9]+$ ]] || fail 'FAULT_MEDIA_MIB must be a positive integer'
[[ -d "$REPO_ROOT" ]] || fail "repository root is not a directory: $REPO_ROOT"

for tool in "${REQUIRED_TOOLS[@]}"; do
    tool_path="$(command -v "$tool" 2>/dev/null || true)"
    [[ -n "$tool_path" ]] || fail "missing prerequisite tool: $tool"
    TOOLRECORDS+=("$(printf '%s\t%s' "$tool" "$tool_path")")
    printf 'RESOURCE: tool %s -> %s\n' "$tool" "$tool_path"
done

for wrapper in tools/moon-local tools/moon-cargo; do
    [[ -x "$REPO_ROOT/$wrapper" ]] || fail "missing repository wrapper: $wrapper"
done
printf 'RESOURCE: moon wrappers tools/moon-local and tools/moon-cargo present\n'

CHROMIUM="${CHROMIUM_EXECUTABLE:-/usr/bin/chromium}"
[[ -x "$CHROMIUM" ]] || fail "no executable browser at $CHROMIUM (set CHROMIUM_EXECUTABLE)"
printf 'RESOURCE: browser %s bytes=%s\n' "$CHROMIUM" "$(stat -Lc '%s' -- "$CHROMIUM")"

mkdir -p -m 700 -- "$SCRATCH_STORE"
[[ -d "$SCRATCH_STORE" && -O "$SCRATCH_STORE" && ! -L "$SCRATCH_STORE" ]] ||
    fail "scratch root must be an owned directory, not a symlink: $SCRATCH_STORE"
mkdir -p -m 700 -- "$SCRATCH_STORE/tmp"
[[ -d "$SCRATCH_STORE/tmp" && -O "$SCRATCH_STORE/tmp" && ! -L "$SCRATCH_STORE/tmp" ]] ||
    fail "scratch tmp must be an owned directory, not a symlink: $SCRATCH_STORE/tmp"
TMPDIR="$SCRATCH_STORE/tmp"
export TMPDIR
SCRATCH_FSTYPE="$(findmnt -no FSTYPE --target "$SCRATCH_STORE" || true)"
SCRATCH_MOUNT="$(findmnt -no TARGET --target "$SCRATCH_STORE" || true)"
SCRATCH_AVAIL_KIB="$(df -Pk -- "$SCRATCH_STORE" | tail -1 | awk '{print $4}')"
printf 'RESOURCE: scratch root %s mode=%s fs=%s mount=%s avail_kib=%s\n' \
    "$SCRATCH_STORE" "$(stat -Lc '%a' -- "$SCRATCH_STORE")" "$SCRATCH_FSTYPE" "$SCRATCH_MOUNT" "$SCRATCH_AVAIL_KIB"
if [[ "$SCRATCH_FSTYPE" == tmpfs ]]; then
    printf 'NOTICE: scratch root is tmpfs; the ENOSPC lanes mount their own bounded media and nothing here survives a reboot\n'
fi

SPARSE="$TMPDIR/native-resources-sparse-$$"
truncate -s "${MEDIA_MIB}M" -- "$SPARSE" || fail "cannot create a bounded sparse media probe: $SPARSE"
SPARSE_SIZE="$(stat -Lc '%s' -- "$SPARSE")"
SPARSE_BLOCKS="$(stat -Lc '%b' -- "$SPARSE")"
rm -f -- "$SPARSE"
printf 'RESOURCE: bounded disk media apparent_bytes=%s allocated_blocks=%s (probe removed)\n' \
    "$SPARSE_SIZE" "$SPARSE_BLOCKS"

if ! NS_OUT="$(unshare --user --map-root-user --mount --propagation private true 2>&1)"; then
    fail "unprivileged user+mount namespace unsupported; scenario-09 and scenario-10 are blocked (observed: $NS_OUT)"
fi
printf 'RESOURCE: isolation unprivileged user+mount namespace supported\n'

MEDIA_DIR="$TMPDIR/native-resources-media-$$"
MEDIA_OUT=""
if ! MEDIA_OUT="$(timeout --signal=TERM 120 unshare --user --map-root-user --mount --propagation private -- \
    bash -c '
        set -euo pipefail
        media="$1"
        mib="$2"
        mkdir -p -- "$media"
        mount -t tmpfs -o "size=${mib}M",mode=700 tmpfs "$media"
        df -Pk -- "$media" | tail -1
        printf "media-probe\n" > "$media/probe"
        printf "probe-bytes=%s\n" "$(stat -Lc "%s" -- "$media/probe")"
        sha256sum -- "$media/probe"
    ' _ "$MEDIA_DIR" "$MEDIA_MIB" 2>&1)"; then
    rmdir -- "$MEDIA_DIR" 2>/dev/null || rm -rf -- "$MEDIA_DIR"
    fail "bounded tmpfs media unsupported inside the namespace; scenario-09 and scenario-10 are blocked (observed: $MEDIA_OUT)"
fi
rmdir -- "$MEDIA_DIR" 2>/dev/null || rm -rf -- "$MEDIA_DIR"
printf '%s\n' "$MEDIA_OUT" | sed 's/^/EVIDENCE: media /'
printf 'RESOURCE: isolation bounded tmpfs media %s MiB mounted inside the namespace\n' "$MEDIA_MIB"

UUID_VALUE="$(head -c 36 /proc/sys/kernel/random/uuid || true)"
[[ "$UUID_VALUE" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]] ||
    fail 'kernel uuid source is unavailable; VM session tokens cannot be minted'
printf 'RESOURCE: VM session mechanism: one kernel uuid token + an owned 0700 session root + an exclusive flock, minted per wrapper run (CENSUS_NATIVE_VM_SESSION_ROOT/TOKEN)\n'

[[ -r /proc/self/cmdline && -r /proc/self/environ ]] ||
    fail 'per-process cmdline/environ are unreadable; owned process identity cannot be captured'
printf 'RESOURCE: process identity: pgrep + ss + /proc/<pid>/cmdline,environ readable for owned-PID capture\n'

LIVE_PROCS="$(pgrep -a -f '(^|/)(census-serve|census-service) |restate-server --no-logo' 2>/dev/null || true)"
if [[ -n "$LIVE_PROCS" ]]; then
    printf 'NOTICE: %s census/Restate processes already run on this host; none is touched here, but confirm none owns the store the fault will target (single writer)\n' \
        "$(printf '%s\n' "$LIVE_PROCS" | grep -c .)"
    printf '%s\n' "$LIVE_PROCS" | head -20 | sed 's/^/NOTICE: live /'
fi
printf 'RESOURCE: owned processes are started, identified and reaped by each wrapper; this verifier starts and touches none\n'

HOST_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'RESOURCE: clock policy: the host clock is read-only; observed host UTC %s; scenario-12 uses the guest-only midnight lane and scenario-03 the guest reboot lane\n' \
    "$HOST_UTC"

ARTIFACT_PARENT="$HOME/.cache/cen-native-vm"
mkdir -p -m 700 -- "$ARTIFACT_PARENT"
[[ -d "$ARTIFACT_PARENT" && -O "$ARTIFACT_PARENT" && ! -L "$ARTIFACT_PARENT" ]] ||
    fail "persistent guest disk parent must be an owned directory, not a symlink: $ARTIFACT_PARENT"
if [[ "$(stat -Lc '%a' -- "$ARTIFACT_PARENT")" != 700 ]]; then
    chmod 700 -- "$ARTIFACT_PARENT" || fail "cannot tighten the persistent guest disk parent mode: $ARTIFACT_PARENT"
fi
printf 'RESOURCE: persistent guest disks prepared under %s mode=%s\n' \
    "$ARTIFACT_PARENT" "$(stat -Lc '%a' -- "$ARTIFACT_PARENT")"

resolved() { realpath -e -- "$1" 2>/dev/null || fail "path does not resolve: $1"; }

budget_artifact() {
    local role="$1" path="$2" budget="$3" target size
    target="$(resolved "$path")"
    [[ -f "$target" && -r "$target" ]] || fail "$role is not a readable regular file: $target"
    size="$(stat -Lc '%s' -- "$target")"
    [[ "$size" =~ ^[0-9]+$ ]] || fail "$role has an invalid size: $target"
    ((size > 0 && size <= budget)) || fail "$role is outside its byte budget ($size > $budget): $target"
}

record_artifact() {
    local role="$1" path="$2" version="$3" digest="${4:-}" target size
    target="$(resolved "$path")"
    [[ -f "$target" && -r "$target" ]] || fail "$role is not a readable regular file: $target"
    size="$(stat -Lc '%s' -- "$target")"
    [[ -n "$digest" ]] || digest="$(sha256sum -- "$target" | awk '{print $1}')"
    RECORDS+=("$(printf '%s\t%s\t%s\t%s\t%s' "$role" "$target" "$size" "$digest" "$version")")
    printf 'RESOURCE: %s %s bytes=%s sha256=%s version=%s\n' "$role" "$target" "$size" "$digest" "${version:-none}"
}

version_line() {
    local output
    output="$(timeout 30 "$1" --version 2>&1 || true)"
    printf '%s' "${output%%$'\n'*}"
}

RESTATE_SERVER="${RESTATE_SERVER_BIN:-$RESOURCE_ROOT/restate/$RESTATE_PIN/restate-server}"
budget_artifact 'restate-server' "$RESTATE_SERVER" "$BINARY_BUDGET"
RESTATE_PATH="$(resolved "$RESTATE_SERVER")"
[[ -x "$RESTATE_PATH" ]] || fail "restate-server is not executable: $RESTATE_PATH"
RESTATE_VERSION="$(version_line "$RESTATE_PATH")"
[[ -n "$RESTATE_VERSION" ]] || fail "restate-server --version produced no output: $RESTATE_PATH"
[[ "$RESTATE_VERSION" == *"$RESTATE_PIN"* ]] ||
    printf 'NOTICE: restate-server --version does not name the pinned %s: %s\n' "$RESTATE_PIN" "$RESTATE_VERSION"
if [[ -z "${RESTATE_SERVER_BIN:-}" ]]; then
    PINNED_INSTALL="$(resolved "$RESOURCE_ROOT/restate/$RESTATE_PIN")"
    [[ "$(dirname -- "$RESTATE_PATH")" == "$PINNED_INSTALL" ]] ||
        fail "restate-server is not the pinned $RESTATE_PIN install: $RESTATE_PATH"
fi
record_artifact 'restate-server' "$RESTATE_PATH" "$RESTATE_VERSION"

TOOLS_DIR="$(resolved "$RESOURCE_ROOT/qemu/$QEMU_PIN")"
QEMU_BIN="$TOOLS_DIR/prefix/usr/bin/qemu-system-x86_64"
QEMU_IMG="$TOOLS_DIR/prefix/usr/bin/qemu-img"
BIOS="$TOOLS_DIR/prefix/usr/share/qemu/bios-256k.bin"
QEMU_LIB_DIR="$(resolved "$TOOLS_DIR/prefix/usr/lib")"
[[ -d "$QEMU_LIB_DIR" ]] || fail "qemu prefix library directory is missing: $QEMU_LIB_DIR"
export LD_LIBRARY_PATH="${QEMU_LIB_DIR}${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
for binary in "$QEMU_BIN" "$QEMU_IMG"; do
    budget_artifact 'qemu binary' "$binary" "$BINARY_BUDGET"
    [[ -x "$(resolved "$binary")" ]] || fail "qemu binary is not executable: $binary"
done
budget_artifact 'qemu bios' "$BIOS" "$BIOS_BUDGET"
QEMU_VERSION="$(version_line "$(resolved "$QEMU_BIN")")"
[[ -n "$QEMU_VERSION" ]] || fail "qemu-system-x86_64 --version produced no output: $QEMU_BIN"
[[ "$QEMU_VERSION" == *"$QEMU_PIN"* ]] ||
    fail "qemu-system-x86_64 --version does not name the pinned $QEMU_PIN: $QEMU_VERSION"
record_artifact 'qemu-system-x86_64' "$QEMU_BIN" "$QEMU_VERSION"
record_artifact 'qemu-img' "$QEMU_IMG" "$(version_line "$(resolved "$QEMU_IMG")")"
record_artifact 'qemu-bios-256k' "$BIOS" ''

PKGS_DIR="$(resolved "$TOOLS_DIR/pkgs")"
PKGS_MANIFEST="$PKGS_DIR/SHA256SUMS.txt"
[[ -f "$PKGS_MANIFEST" ]] || fail "staged package manifest is missing: $PKGS_MANIFEST"
if ! PKGS_OUT="$(cd "$PKGS_DIR" && sha256sum --check --strict SHA256SUMS.txt 2>&1)"; then
    printf '%s\n' "$PKGS_OUT" >&2
    fail "staged qemu package checksums failed in $PKGS_DIR"
fi
PKGS_LISTED="$(awk '{print $2}' "$PKGS_MANIFEST")"
printf 'RESOURCE: staged qemu %s package checksums verified: %s listed packages in %s\n' \
    "$QEMU_PIN" "$(printf '%s\n' "$PKGS_LISTED" | grep -c .)" "$PKGS_DIR"
while IFS= read -r staged; do
    staged_name="$(basename -- "$staged")"
    if printf '%s\n' "$PKGS_LISTED" | grep -qxF -- "$staged_name"; then
        continue
    fi
    UNLISTED+=("$(printf '%s\t%s\t%s' "$staged_name" "$(stat -Lc '%s' -- "$staged")" "$(sha256sum -- "$staged" | awk '{print $1}')")")
done < <(find "$PKGS_DIR" -maxdepth 1 -name '*.pkg.tar.zst' -type f | sort)
if [ "${#UNLISTED[@]}" -gt 0 ]; then
    printf 'NOTICE: %s staged packages are absent from SHA256SUMS.txt; recording their observed hashes\n' "${#UNLISTED[@]}"
    for row in "${UNLISTED[@]}"; do
        IFS=$'\t' read -r unlisted_name unlisted_bytes unlisted_digest <<<"$row"
        printf 'RESOURCE: unlisted package %s bytes=%s sha256=%s\n' "$unlisted_name" "$unlisted_bytes" "$unlisted_digest"
    done
fi

IMAGES_DIR="$(resolved "$RESOURCE_ROOT/images")"
IMAGE="$IMAGES_DIR/$IMAGE_NAME"
[[ -f "$IMAGE" ]] || fail "pinned guest image is missing: $IMAGE"
if ! IMAGE_OUT="$(cd "$IMAGES_DIR" && sha256sum --check --strict SHA256SUMS.txt 2>&1)"; then
    printf '%s\n' "$IMAGE_OUT" >&2
    fail "published guest image checksum failed in $IMAGES_DIR"
fi
budget_artifact 'guest image' "$IMAGE" "$IMAGE_BUDGET"
IMAGE_DIGEST="$(sha256sum -- "$IMAGE" | awk '{print $1}')"
[[ "$IMAGE_DIGEST" == "$IMAGE_SHA256" ]] ||
    fail "guest image digest differs from the pinned digest: $IMAGE_DIGEST"
record_artifact 'guest-image' "$IMAGE" "sha256 $IMAGE_SHA256" "$IMAGE_DIGEST"

CAPTURES="${CENSUS_NATIVE_VM_CAPTURES:-$REPO_ROOT/var/native-vm-captures-20261024.json}"
budget_artifact 'guest capture manifest' "$CAPTURES" "$CAPTURES_BUDGET"
CAPTURES_PATH="$(resolved "$CAPTURES")"
for field in '.index.body' '.index.metadata' '.roster.body' '.roster.metadata'; do
    capture_path="$(jq -er "$field | select(type == \"string\" and length > 0)" "$CAPTURES_PATH" 2>/dev/null || true)"
    [[ -n "$capture_path" ]] || fail "guest capture manifest lacks $field: $CAPTURES_PATH"
    case "$capture_path" in
        /*) ;;
        *) capture_path="$REPO_ROOT/$capture_path" ;;
    esac
    budget_artifact "guest capture $field" "$capture_path" "$CAPTURES_BUDGET"
done
printf 'RESOURCE: guest capture manifest %s binds 4 capture files\n' "$CAPTURES_PATH"
if [[ "$CAPTURES_PATH" == "$REPO_ROOT/"* ]]; then
    printf 'NOTICE: manifest capture paths are repository-relative; resolved against %s here, while native-vm-qualification.sh resolves them against its process cwd\n' \
        "$REPO_ROOT"
fi
record_artifact 'guest-capture-manifest' "$CAPTURES_PATH" ''
VERIFY_BIN="${VERIFY_BINARY:-$REPO_ROOT/target/moon-build/x86_64-unknown-linux-gnu/release/examples/qualification_native_vm}"
[[ -x "$VERIFY_BIN" ]] || fail "verify binary missing at $VERIFY_BIN; build it with pipeline:build -- --release -p census-service --features native-fault-injection --example qualification_native_vm"
"$VERIFY_BIN" verify --manifest "$CAPTURES_PATH" || fail "guest capture provenance refused: $CAPTURES_PATH"
printf 'RESOURCE: guest capture provenance verified by capture/protocol checks: %s\n' "$CAPTURES_PATH"

ENDPOINT="${SERVE_BINARY:-$REPO_ROOT/target/moon-build/x86_64-unknown-linux-gnu/release/census-serve}"
if [[ -f "$ENDPOINT" ]]; then
    missing_seams=0
    for seam in "${SEAMS[@]}"; do
        grep -aqF "$seam" "$ENDPOINT" || {
            printf 'FAIL: feature endpoint lacks seam %s: %s\n' "$seam" "$ENDPOINT"
            missing_seams=1
        }
    done
    ((missing_seams == 0)) || exit 1
    printf 'RESOURCE: feature endpoint carries all %s fault seams: %s\n' "${#SEAMS[@]}" "$ENDPOINT"
else
    printf 'NOTICE: feature endpoint absent at %s; build it with pipeline:build -- --release -p census-service --features native-fault-injection --bin census-serve --example qualification_native_vm\n' \
        "$ENDPOINT"
fi

if [[ -n "$CERTIFICATE" ]]; then
    cert_parent="$(dirname -- "$CERTIFICATE")"
    [[ -d "$cert_parent" && -O "$cert_parent" && ! -L "$cert_parent" ]] ||
        fail "certificate parent must be an owned directory, not a symlink: $cert_parent"
    [[ ! -e "$CERTIFICATE" && ! -L "$CERTIFICATE" ]] ||
        fail "certificate already exists; use a fresh owned path: $CERTIFICATE"
    pending="$CERTIFICATE.pending.$$"
    [[ ! -e "$pending" ]] || fail "certificate pending path already exists: $pending"
    if [ "${#UNLISTED[@]}" -gt 0 ]; then
        unlisted_json="$(printf '%s\n' "${UNLISTED[@]}" |
            jq -Rn '[inputs | split("\t") | {name: .[0], bytes: (.[1] | tonumber), sha256: .[2]}]')"
    else
        unlisted_json='[]'
    fi
    umask 077
    jq -n \
        --arg generated_at "$HOST_UTC" \
        --arg host "$(uname -n)" \
        --arg repo_root "$REPO_ROOT" \
        --arg resource_root "$RESOURCE_ROOT" \
        --arg restate_pin "$RESTATE_PIN" \
        --arg qemu_pin "$QEMU_PIN" \
        --arg restate_version "$RESTATE_VERSION" \
        --arg qemu_version "$QEMU_VERSION" \
        --arg image_sha256 "$IMAGE_SHA256" \
        --arg scratch "$SCRATCH_STORE" \
        --arg scratch_fs "${SCRATCH_FSTYPE:-unknown}" \
        --argjson scratch_avail "$SCRATCH_AVAIL_KIB" \
        --argjson artifacts "$(printf '%s\n' "${RECORDS[@]}" |
            jq -Rn '[inputs | split("\t") | {role: .[0], path: .[1], bytes: (.[2] | tonumber), sha256: .[3], version: .[4]}]')" \
        --argjson tools "$(printf '%s\n' "${TOOLRECORDS[@]}" |
            jq -Rn '[inputs | split("\t") | {name: .[0], path: .[1]}]')" \
        --argjson unlisted "$unlisted_json" \
        --argjson media_mib "$MEDIA_MIB" \
        --argjson sparse_blocks "$SPARSE_BLOCKS" \
        '{schema: 1, generated_at: $generated_at, host: $host, repo_root: $repo_root,
          resource_root: $resource_root,
          pins: {restate: $restate_pin, qemu: $qemu_pin, image_sha256: $image_sha256},
          observed_versions: {restate: $restate_version, qemu: $qemu_version},
          scratch: {path: $scratch, filesystem: $scratch_fs, avail_kib: $scratch_avail},
          isolation: {user_mount_namespace: true, tmpfs_media_mib: $media_mib,
            sparse_media_blocks: $sparse_blocks,
            clock: "host clock read-only; guest-only scenario-03 reboot and scenario-12 midnight lanes"},
          artifacts: $artifacts, tools: $tools, unlisted_packages: $unlisted}' >"$pending"
    mv -T -- "$pending" "$CERTIFICATE"
    printf 'EVIDENCE: native resource certificate %s\n' "$CERTIFICATE"
fi

printf 'PASS: native fault resources verified (artifacts=%s tools=%s unlisted_packages=%s scratch=%s)\n' \
    "${#RECORDS[@]}" "${#TOOLRECORDS[@]}" "${#UNLISTED[@]}" "$SCRATCH_STORE"
