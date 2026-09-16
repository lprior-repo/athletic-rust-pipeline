#!/usr/bin/env bash
# Run pinned native Restate with persistent, private user-local storage.
# This launcher never removes, truncates, or recreates the persistent data path.
set -euo pipefail

readonly ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/athletic-rust-pipeline/restate"
readonly SERVER="$ROOT/1.7.10/restate-server"
readonly CONFIG="$ROOT/restate.toml"
readonly DATA="$ROOT/data"

usage() {
    printf 'Usage: %s {start|version|help}\n' "$0" >&2
}

case "${1:-}" in
    version)
        exec "$SERVER" --version
        ;;
    help)
        exec "$SERVER" --help
        ;;
    start)
        if [[ $# -ne 1 ]]; then
            usage
            exit 64
        fi
        if [[ ! -x "$SERVER" ]]; then
            printf 'error: missing executable: %s\n' "$SERVER" >&2
            exit 1
        fi
        if [[ ! -f "$CONFIG" || -L "$CONFIG" ]]; then
            printf 'error: configuration is missing or a symlink: %s\n' "$CONFIG" >&2
            exit 1
        fi
        if [[ ! -d "$DATA" || -L "$DATA" ]]; then
            printf 'error: persistent data directory is missing or a symlink: %s\n' "$DATA" >&2
            exit 1
        fi
        # Strip inherited RESTATE_* overrides so ports, node identity, and base-dir
        # remain the reviewed values in the pinned configuration file. The server
        # receives SIGTERM directly because it is the exec'd process.
        exec env -i \
            PATH="${PATH:-/usr/bin:/bin}" \
            HOME="${HOME:-/tmp}" \
            "$SERVER" --no-logo --config-file="$CONFIG"
        ;;
    *)
        usage
        exit 64
        ;;
esac
