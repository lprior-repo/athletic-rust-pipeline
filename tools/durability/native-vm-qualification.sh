#!/usr/bin/env bash
set -euo pipefail

native_vm_fail() {
    printf 'FAIL: native VM qualification: %s\n' "$*" >&2
    exit 1
}

native_vm_file() {
    local path="$1" limit="$2" size
    [[ -f "$path" && -r "$path" ]] || native_vm_fail "missing readable file: $path"
    size=$(stat -Lc '%s' -- "$path")
    [[ "$size" =~ ^[0-9]+$ ]] || native_vm_fail "invalid file size: $path"
    (( size > 0 && size <= limit )) || native_vm_fail "file outside byte budget: $path ($size)"
}

native_vm_inputs() {
    local field path
    HOST=$(realpath -e -- "${CENSUS_NATIVE_VM_HOST:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/examples/qualification_native_vm}")
    SERVE=$(realpath -e -- "${SERVE_BINARY:-$REPO_ROOT/target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve}")
    NODE=$(realpath -e -- "${RESTATE_SERVER_BIN:-${RESTATE_BINARY:-$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server}}")
    TOOLS=$(realpath -e -- "$HOME/.local/share/athletic-rust-pipeline/qemu/11.1.1")
    IMAGE=$(realpath -e -- "$HOME/.local/share/athletic-rust-pipeline/images/Arch-Linux-x86_64-cloudimg-20261001.604814.qcow2")
    [[ -n "${CENSUS_NATIVE_VM_CAPTURES:-}" ]] || native_vm_fail 'CENSUS_NATIVE_VM_CAPTURES is required'
    CAPTURES=$(realpath -e -- "$CENSUS_NATIVE_VM_CAPTURES")
    for path in "$HOST" "$SERVE" "$NODE" "$TOOLS/prefix/usr/bin/qemu-system-x86_64" "$TOOLS/prefix/usr/bin/qemu-img"; do
        [[ -x "$path" ]] || native_vm_fail "missing executable: $path"
        native_vm_file "$path" 536870912
    done
    native_vm_file "$IMAGE" 8589934592
    native_vm_file "$CAPTURES" 33554432
    INPUT_FILES=("$HOST" "$SERVE" "$NODE" "$CAPTURES")
    for field in '.index.body' '.index.metadata' '.roster.body' '.roster.metadata'; do
        path=$(jq -er "$field | select(type == \"string\" and length > 0 and (contains(\"\n\") | not) and (contains(\"\r\") | not))" "$CAPTURES")
        path=$(realpath -e -- "$path")
        native_vm_file "$path" 33554432
        INPUT_FILES+=("$path")
    done
    INPUT_FILES+=("$TOOLS/prefix/usr/bin/qemu-system-x86_64" "$TOOLS/prefix/usr/bin/qemu-img" "$TOOLS/prefix/usr/share/qemu/bios-256k.bin" "$IMAGE")
    native_vm_file "$TOOLS/prefix/usr/share/qemu/bios-256k.bin" 33554432
}

native_vm_session() {
    if [[ ! ${CENSUS_NATIVE_VM_SESSION_ROOT+x} && ! ${CENSUS_NATIVE_VM_SESSION_TOKEN+x} ]]; then
        CENSUS_NATIVE_VM_SESSION_ROOT=$(mktemp -d /tmp/cen-vm-session-XXXXXXXX)
        CENSUS_NATIVE_VM_SESSION_TOKEN=$(< /proc/sys/kernel/random/uuid)
    fi
    [[ -n "${CENSUS_NATIVE_VM_SESSION_ROOT:-}" && -n "${CENSUS_NATIVE_VM_SESSION_TOKEN:-}" ]] || native_vm_fail 'session root and token must both be provided or both absent'
    TOKEN="$CENSUS_NATIVE_VM_SESSION_TOKEN"
    [[ "$TOKEN" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]] || native_vm_fail 'malformed fresh session token'
    SESSION=$(realpath -e -- "$CENSUS_NATIVE_VM_SESSION_ROOT")
    [[ -d "$SESSION" && -O "$SESSION" && ! -L "$CENSUS_NATIVE_VM_SESSION_ROOT" ]] || native_vm_fail 'session root must be an owned directory, not a symlink'
    [[ $(stat -Lc '%a' -- "$SESSION") == 700 ]] || native_vm_fail 'session root must have mode 0700'
    SESSION="$SESSION/native-vm"
    if mkdir -m 700 -- "$SESSION"; then
        printf '%s\n' "$TOKEN" > "$SESSION/token"
    else
        [[ -d "$SESSION" && -O "$SESSION" && ! -L "$SESSION" ]] || native_vm_fail 'invalid existing session evidence directory'
        native_vm_file "$SESSION/token" 37
        [[ $(< "$SESSION/token") == "$TOKEN" ]] || native_vm_fail 'historical or different harness session rejected'
    fi
    exec 9> "$SESSION/lock"
    flock -n 9 || native_vm_fail 'another wrapper owns this session'
    printf 'EVIDENCE: native VM session %s\n' "$SESSION"
}

native_vm_output_files() {
    OUTPUT_FILES=("$ROOT/qualification-certificates.json" "$ROOT/verdict.json" "$ROOT/cleanup.json" "$ROOT/manifest.json" "$ROOT/measured-durability-oracles.json")
    local path
    for path in "${OUTPUT_FILES[@]}"; do
        native_vm_file "$path" 33554432
    done
    jq -e '.verdict == "PASS" and .failure == null and .signal_reason == 0 and .certificate == "qualification-certificates.json"' "$ROOT/verdict.json" > /dev/null
    jq -e 'type == "object" and (.scenario03 | type == "object") and (.scenario12 | type == "object") and (.cleanup | type == "object") and (.acknowledged_effects | type == "object")' "$ROOT/qualification-certificates.json" > /dev/null
    OUTPUT_FILES+=("$SESSION/producer.stdout" "$SESSION/producer.stderr" "$SESSION/producer.exit")
}

native_vm_artifact_parent() {
    local parent="$HOME/.cache/cen-native-vm"
    mkdir -p -m 700 -- "$parent"
    [[ -d "$parent" && -O "$parent" && ! -L "$parent" ]] || native_vm_fail 'persistent artifact parent must be an owned directory, not a symlink'
    [[ $(stat -Lc '%a' -- "$parent") == 700 ]] || native_vm_fail 'persistent artifact parent must have mode 0700'
    realpath -e -- "$parent"
}

native_vm_reuse() {
    native_vm_file "$SESSION/completion.json" 4096
    jq -e --arg token "$TOKEN" '.schema == 1 and .session_token == $token and .producer_exit == 0' "$SESSION/completion.json" > /dev/null
    ROOT=$(jq -er '.producer_root | select(type == "string")' "$SESSION/completion.json")
    local parent
    parent=$(native_vm_artifact_parent)
    [[ "$ROOT" == "$parent"/run-????????/machine && -d "$ROOT" && -O "$ROOT" && ! -L "$ROOT" ]] || native_vm_fail 'invalid preserved persistent producer root'
    [[ $(realpath -e -- "$ROOT") == "$ROOT" ]] || native_vm_fail 'producer root has a symlinked ancestor'
    [[ $(realpath -e -- "$SESSION/producer-root") == "$ROOT" ]] || native_vm_fail 'producer evidence link changed'
    [[ $(< "$SESSION/producer.exit") == 0 ]] || native_vm_fail 'original producer did not exit zero'
    cmp -s -- "$REQUEST" "$SESSION/inputs.sha256" || native_vm_fail 'current host, serve, node, manifest, captures, QEMU or image differ from completed producer'
    sha256sum --check --status "$SESSION/inputs.sha256" || native_vm_fail 'completed producer inputs changed'
    sha256sum --check --status "$SESSION/outputs.sha256" || native_vm_fail 'completed producer evidence changed or incomplete'
    native_vm_output_files
    printf 'EVIDENCE: reusing exact exit-0 producer in this fresh session: %s\n' "$ROOT"
}

native_vm_produce() {
    [[ ! -e "$SESSION/completion.json" && ! -e "$SESSION/producer-root" && ! -L "$SESSION/producer-root" ]] || native_vm_fail 'scenario03 requires one new producer; existing or incomplete producer cannot be overwritten'
    local allocation parent rc=0
    parent=$(native_vm_artifact_parent)
    allocation=$(mktemp -d "$parent/run-XXXXXXXX")
    ROOT="$allocation/machine"
    ln -s -- "$ROOT" "$SESSION/producer-root"
    cp -- "$REQUEST" "$SESSION/inputs.sha256"
    printf 'EVIDENCE: preserving native VM disks and logs at %s\n' "$ROOT"
    printf '%q ' "$HOST" host --root "$ROOT" --tools "$TOOLS" --base-image "$IMAGE" --census-serve "$SERVE" --restate "$NODE" --captures "$CAPTURES" > "$SESSION/producer.command"
    printf '\n' >> "$SESSION/producer.command"
    timeout --foreground --signal=TERM 10800 "$HOST" host --root "$ROOT" --tools "$TOOLS" --base-image "$IMAGE" --census-serve "$SERVE" --restate "$NODE" --captures "$CAPTURES" > "$SESSION/producer.stdout" 2> "$SESSION/producer.stderr" || rc=$?
    printf '%s\n' "$rc" > "$SESSION/producer.exit"
    cat -- "$SESSION/producer.stdout"
    cat -- "$SESSION/producer.stderr" >&2
    (( rc == 0 )) || native_vm_fail "actual producer exited $rc; disks/logs retained at $ROOT"
    sha256sum --check --status "$SESSION/inputs.sha256" || native_vm_fail 'producer inputs changed during qualification'
    native_vm_output_files
    sha256sum -- "${OUTPUT_FILES[@]}" > "$SESSION/outputs.sha256"
    jq -n --arg token "$TOKEN" --arg root "$ROOT" '{schema:1,session_token:$token,producer_exit:0,producer_root:$root}' > "$SESSION/completion.pending.json"
    mv -T -- "$SESSION/completion.pending.json" "$SESSION/completion.json"
}

native_vm_qualification() {
    local scenario="$1" command
    [[ "$scenario" == 03 || "$scenario" == 12 ]] || native_vm_fail 'unknown wrapper scenario'
    for command in realpath stat jq sha256sum cmp mktemp mkdir flock timeout ln cp mv cat; do
        command -v "$command" > /dev/null || native_vm_fail "required tool absent: $command"
    done
    native_vm_session
    native_vm_inputs
    local REQUEST="$SESSION/scenario-$scenario-current.sha256"
    [[ ! -e "$REQUEST" ]] || native_vm_fail 'this scenario has already entered the session; require a fresh harness invocation'
    sha256sum -- "${INPUT_FILES[@]}" > "$REQUEST"
    if [[ "$scenario" == 12 && -f "$SESSION/completion.json" ]]; then
        native_vm_reuse
    else
        native_vm_produce
    fi
    printf 'EVIDENCE: strict Rust certificates %s/qualification-certificates.json\n' "$ROOT"
    case "$scenario" in
        03) printf 'PASS: catalog03 actual original reserved source invocation recovered across native VM reset; acknowledged effects preserved (not full national census or all HTTP subphases)\n' ;;
        12) printf 'PASS: catalog12 two fresh production FULL200 HTTPS acquisitions bracketed natural guest midnight with stable identities\n' ;;
    esac
}
