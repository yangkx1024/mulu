#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

die() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info() { printf '\033[36m>>\033[0m %s\n' "$*"; }

[[ "$(uname)" == "Darwin" ]] || die "packaging must run on macOS"

for cmd in cargo cargo-packager codesign xcrun security; do
    command -v "$cmd" >/dev/null 2>&1 || die "missing required command: $cmd"
done

[[ -f .env ]] || die ".env not found (expected APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID, APPLE_SIGNING_IDENTITY)"

set -a
# shellcheck disable=SC1091
source .env
set +a

for var in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_SIGNING_IDENTITY; do
    [[ -n "${!var:-}" ]] || die "$var is not set in .env"
done

if ! security find-identity -v -p codesigning | grep -qF "$APPLE_SIGNING_IDENTITY"; then
    die "signing identity not found in keychain: $APPLE_SIGNING_IDENTITY"
fi

grep -qE '^\[package\.metadata\.packager\.macos\]' Cargo.toml \
    || die "Cargo.toml missing [package.metadata.packager.macos] section"

# cargo-packager's `-c` replaces the config instead of merging, so inject
# signing-identity directly into Cargo.toml and restore it on exit.
cp Cargo.toml Cargo.toml.pkg-bak
trap 'mv Cargo.toml.pkg-bak Cargo.toml' EXIT INT TERM

awk -v id="$APPLE_SIGNING_IDENTITY" '
    { print }
    /^\[package\.metadata\.packager\.macos\][[:space:]]*$/ {
        printf "signing-identity = \"%s\"\n", id
    }
' Cargo.toml.pkg-bak > Cargo.toml

HOST_TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"

info "host target:    $HOST_TARGET"
info "signing id:     $APPLE_SIGNING_IDENTITY"
info "apple id:       $APPLE_ID"
info "team id:        $APPLE_TEAM_ID"
info "starting cargo-packager..."

cargo packager --release "$@"

info "done"
