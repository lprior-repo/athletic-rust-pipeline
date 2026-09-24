#!/usr/bin/env bash
# Deployment lifecycle helper for the athletic-rust-pipeline.
#
# Implements the mechanical half of the manual lifecycle described in
# docs/deployment-lifecycle.md:
#
#   install   <git-sha> <port>   — install the binary, print the register command
#   list                          — list registered deployments with active-invocation counts
#
# Rules:
#   - Refuses to install over an existing /releases/<sha>/ (immutability).
#   - Refuses a port that is already serving (netstat/ss check).
#   - Requires the restate CLI to be on PATH.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RELEASES_DIR="$REPO_ROOT/var/releases"
RESTATE_ADMIN_PORT=19095
RESTATE_ADMIN_HOST="127.0.0.1"

# ── helpers ──────────────────────────────────────────────────────────────────

die() { printf "%s\n" "$1" >&2; exit 1; }

check_restate_cli() {
    if ! command -v restate &>/dev/null; then
        die "restate CLI not found on PATH. Install it before running this script."
    fi
    if ! restate --version &>/dev/null; then
        die "restate CLI is present but reports an error — check your installation."
    fi
}

# ── install ──────────────────────────────────────────────────────────────────

cmd_install() {
    local sha="${1:-}"
    local port="${2:-}"

    if [[ -z "$sha" ]]; then
        die "Usage: deploy-lifecycle.sh install <git-sha> <port>"
    fi
    if [[ -z "$port" ]]; then
        die "Usage: deploy-lifecycle.sh install <git-sha> <port>"
    fi

    # Validate sha is a full 40-char hash
    if [[ ! "$sha" =~ ^[0-9a-f]{40}$ ]]; then
        die "git-sha must be a full 40-character hex hash, got: $sha"
    fi

    # Validate port is a number in the allowed range
    if ! [[ "$port" =~ ^[0-9]+$ ]]; then
        die "port must be a number, got: $port"
    fi
    if (( port < 1024 || port > 65535 )); then
        die "port must be between 1024 and 65535, got: $port"
    fi

    # Immutability check: refuse if /releases/<sha>/ already exists
    local release_path="$RELEASES_DIR/$sha"
    if [[ -d "$release_path" ]]; then
        die "REFUSED: $release_path already exists. A registered version's binary is immutable — build to a new SHA."
    fi

    # Port-in-use check
    if ss -tlnp 2>/dev/null | grep -q ":${port} " || netstat -tlnp 2>/dev/null | grep -q ":${port} "; then
        die "REFUSED: port $port is already in use by another service."
    fi

    # Create the release directory
    mkdir -p "$release_path" || die "Failed to create $release_path"

    # Copy the binary from the repo's release build
    local bin_src="$REPO_ROOT/target/release/census-service"
    local bin_dst="$release_path/census-service"
    if [[ ! -f "$bin_src" ]]; then
        die "Binary not found at $bin_src. Run 'cargo build --release' first."
    fi

    cp "$bin_src" "$bin_dst" || die "Failed to copy binary to $bin_dst"
    chmod +x "$bin_dst"

    # Print the register command
    local reg_uri="http://127.0.0.1:${port}/"
    printf "Register the new deployment with:\n"
    printf "  restate deployment register --name census-service-%s --uri %s\n" "$sha" "$reg_uri"

    # List all registered deployments with active-invocation counts
    printf "\nRegistered deployments:\n"
    cmd_list
}

# ── list ──────────────────────────────────────────────────────────────────────

cmd_list() {
    check_restate_cli

    local admin_url="http://${RESTATE_ADMIN_HOST}:${RESTATE_ADMIN_PORT}"

    # Fetch deployments and print them with active-invocation counts
    local json
    json="$(restate deployment list --json 2>/dev/null)" || {
        die "Failed to fetch deployments from $admin_url. Is the Restate admin API reachable?"
    }

    if [[ -z "$json" ]]; then
        printf "No deployments registered.\n"
        return 0
    fi

    printf "%-64s %-20s %s\n" "DEPLOYMENT" "URI" "ACTIVE INVOCATIONS"
    printf "%-64s %-20s %s\n" "----------" "---" "------------------"

    # Parse JSON — each entry has id, uri, and invocation_count
    # restate deployment list --json produces an array of objects
    local entries
    entries="$(echo "$json" | jq -c '.[] // .')" 2>/dev/null || {
        die "Failed to parse deployment list JSON."
    }

    local has_entries=0
    while IFS= read -r entry; do
        has_entries=1
        local id uri invocations
        id="$(echo "$entry" | jq -r '.id // .name // "unknown"' 2>/dev/null)"
        uri="$(echo "$entry" | jq -r '.uri // .address // "unknown"' 2>/dev/null)"
        invocations="$(echo "$entry" | jq -r '.invocation_count // .active_invocations // 0' 2>/dev/null)"

        # If the top-level is an object with a deployments array
        if [[ -z "$id" ]] || [[ "$id" == "null" ]]; then
            id="$(echo "$entry" | jq -r '.deployments[0].id // "unknown"' 2>/dev/null)"
            uri="$(echo "$entry" | jq -r '.deployments[0].uri // "unknown"' 2>/dev/null)"
            invocations="$(echo "$entry" | jq -r '.deployments[0].invocation_count // .deployments[0].active_invocations // 0' 2>/dev/null)"
        fi

        printf "%-64s %-20s %s\n" "$id" "$uri" "$invocations"
    done <<< "$entries"

    if (( has_entries == 0 )); then
        printf "No deployments registered.\n"
    fi
}

# ── describe ──────────────────────────────────────────────────────────────────

cmd_describe() {
    local id="${1:-}"
    if [[ -z "$id" ]]; then
        die "Usage: deploy-lifecycle.sh describe <deployment-id>"
    fi
    check_restate_cli

    restate deployment describe "$id" --extra 2>/dev/null || {
        die "Failed to describe deployment $id. Is it registered?"
    }
}

# ── help ──────────────────────────────────────────────────────────────────────

usage() {
    cat <<'EOF'
Usage: deploy-lifecycle.sh <command> [args]

Commands:
  install <git-sha> <port>
      Install the release binary into /releases/<git-sha>/ and print the
      restate deployment register command. Refuses if the path exists or the
      port is already in use.

  list
      List all registered deployments with their URIs and active invocation
      counts. This is the input for the drain decision: wait until the old
      deployment shows 0 active invocations before shutting it down.

  describe <deployment-id>
      Show detailed info about a deployment, including active invocation
      count. Equivalent to "restate deployment describe <id> --extra".

Environment:
  RELEASES_DIR    Override the releases directory (default: /releases).
                  Not recommended on production machines.
EOF
}

# ── main ──────────────────────────────────────────────────────────────────────

case "${1:-}" in
    install)  shift; cmd_install "$@" ;;
    list)     cmd_list ;;
    describe) shift; cmd_describe "$@" ;;
    --help|-h|"") usage ;;
    *) die "Unknown command: ${1:-}. Run with --help for usage." ;;
esac
