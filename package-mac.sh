#!/usr/bin/env bash
# Build a macOS distribution of Mulu.
#
# Usage:
#   ./package-mac.sh [direct|mas] [cargo-packager args...]   (default: direct)
#
# Modes:
#   direct — Developer ID signed + notarized .dmg via cargo-packager,
#            for GitHub release downloads.
#            Required env: APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID,
#                          APPLE_SIGNING_IDENTITY
#
#   mas    — Mac App Store .pkg (sandboxed, MAS distribution-signed,
#            installer-signed). Built with the `updater` Cargo feature off so
#            the in-app GitHub update check is compiled out (MAS guideline
#            2.4.5). Output: target/mas/Mulu.pkg.
#            Required env: APPLE_TEAM_ID,
#                          MAS_APP_SIGNING_IDENTITY,
#                          MAS_INSTALLER_SIGNING_IDENTITY,
#                          MAS_PROVISIONING_PROFILE
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

die() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info() { printf '\033[36m>>\033[0m %s\n' "$*"; }

MODE="direct"
case "${1:-}" in
    direct|mas) MODE="$1"; shift ;;
    -h|--help|help) sed -n '2,18p' "$0"; exit 0 ;;
esac

[[ "$(uname)" == "Darwin" ]] || die "packaging must run on macOS"

tools=(cargo cargo-packager codesign xcrun security)
[[ "$MODE" == "mas" ]] && tools+=(productbuild plutil awk)
for cmd in "${tools[@]}"; do
    command -v "$cmd" >/dev/null 2>&1 || die "missing required command: $cmd"
done

if [[ -f .env ]]; then
    set -a
    # shellcheck disable=SC1091
    source .env
    set +a
fi

if [[ "$MODE" == "direct" ]]; then
    required=(APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_SIGNING_IDENTITY)
else
    required=(APPLE_TEAM_ID MAS_APP_SIGNING_IDENTITY MAS_INSTALLER_SIGNING_IDENTITY MAS_PROVISIONING_PROFILE)
fi
for var in "${required[@]}"; do
    [[ -n "${!var:-}" ]] || die "$var is not set (export it or add it to .env)"
done

if [[ "$MODE" == "direct" ]]; then
    security find-identity -v -p codesigning | grep -qF "$APPLE_SIGNING_IDENTITY" \
        || die "signing identity not found in keychain: $APPLE_SIGNING_IDENTITY"
else
    [[ -f "$MAS_PROVISIONING_PROFILE" ]] || die "provisioning profile not found: $MAS_PROVISIONING_PROFILE"
    identities="$(security find-identity -v)"
    grep -qF "$MAS_APP_SIGNING_IDENTITY" <<< "$identities" \
        || die "app signing identity not found in keychain: $MAS_APP_SIGNING_IDENTITY"
    grep -qF "$MAS_INSTALLER_SIGNING_IDENTITY" <<< "$identities" \
        || die "installer signing identity not found in keychain: $MAS_INSTALLER_SIGNING_IDENTITY"
fi

grep -qE '^\[package\.metadata\.packager\.macos\][[:space:]]*$' Cargo.toml \
    || die "Cargo.toml missing [package.metadata.packager.macos] section"

# cargo-packager's `-c` replaces the config instead of merging, so we rewrite
# Cargo.toml in place and restore it on exit.
cp Cargo.toml Cargo.toml.pkg-bak
trap 'mv Cargo.toml.pkg-bak Cargo.toml' EXIT INT TERM

HOST_TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"
info "mode:           $MODE"
info "host target:    $HOST_TARGET"

if [[ "$MODE" == "direct" ]]; then
    info "signing id:     $APPLE_SIGNING_IDENTITY"
    info "apple id:       $APPLE_ID"
    info "team id:        $APPLE_TEAM_ID"

    awk -v id="$APPLE_SIGNING_IDENTITY" '
        { print }
        /^\[package\.metadata\.packager\.macos\][[:space:]]*$/ {
            printf "signing-identity = \"%s\"\n", id
        }
    ' Cargo.toml.pkg-bak > Cargo.toml

    info "starting cargo-packager..."
    cargo packager --release "$@"
    info "done"
    exit 0
fi

info "app signing id: $MAS_APP_SIGNING_IDENTITY"
info "installer id:   $MAS_INSTALLER_SIGNING_IDENTITY"
info "team id:        $APPLE_TEAM_ID"
info "provisioning:   $MAS_PROVISIONING_PROFILE"

OUT_DIR="target/mas"
ENTITLEMENTS="$OUT_DIR/Mulu.entitlements"
APP_PATH="$OUT_DIR/Mulu.app"
PKG_PATH="$OUT_DIR/Mulu.pkg"

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

info "building release binary (--no-default-features)..."
cargo build --release --no-default-features

# before-packaging-command is overridden to `true` so cargo-packager doesn't
# rebuild with default features; signing-identity is stripped so MAS certs
# below are the only ones that sign.
awk '
    /^\[package\.metadata\.packager\][[:space:]]*$/ { in_top = 1; in_macos = 0; print; next }
    /^\[package\.metadata\.packager\.macos\][[:space:]]*$/ { in_macos = 1; in_top = 0; print; next }
    /^\[/ { in_top = 0; in_macos = 0 }
    in_top && /^before-packaging-command[[:space:]]*=/ { print "before-packaging-command = \"true\""; next }
    in_top && /^formats[[:space:]]*=/ { print "formats = [\"app\"]"; next }
    in_macos && /^signing-identity[[:space:]]*=/ { next }
    { print }
' Cargo.toml.pkg-bak > Cargo.toml

info "assembling Mulu.app via cargo-packager..."
cargo packager --release --out-dir "$OUT_DIR"
[[ -d "$APP_PATH" ]] || die "expected $APP_PATH after cargo-packager"

INFO_PLIST="$APP_PATH/Contents/Info.plist"
if ! plutil -extract LSApplicationCategoryType raw "$INFO_PLIST" >/dev/null 2>&1; then
    info "injecting LSApplicationCategoryType..."
    plutil -insert LSApplicationCategoryType -string "public.app-category.utilities" "$INFO_PLIST"
fi

info "embedding provisioning profile..."
cp "$MAS_PROVISIONING_PROFILE" "$APP_PATH/Contents/embedded.provisionprofile"

info "stripping extended attributes (incl. com.apple.quarantine)..."
xattr -cr "$APP_PATH"

info "writing entitlements..."
cat > "$ENTITLEMENTS" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.application-identifier</key>
    <string>${APPLE_TEAM_ID}.net.yangkx.mulu</string>
    <key>com.apple.developer.team-identifier</key>
    <string>${APPLE_TEAM_ID}</string>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.device.usb</key>
    <true/>
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
</dict>
</plist>
EOF

info "signing app bundle..."
codesign --force --sign "$MAS_APP_SIGNING_IDENTITY" \
    --entitlements "$ENTITLEMENTS" \
    "$APP_PATH"
codesign --verify --strict --verbose=2 "$APP_PATH"

info "building signed .pkg..."
productbuild \
    --component "$APP_PATH" /Applications \
    --sign "$MAS_INSTALLER_SIGNING_IDENTITY" \
    "$PKG_PATH"

info "done: $PKG_PATH"
info ""
info "upload with Transporter.app, or:"
info "  xcrun altool --upload-app -f $PKG_PATH -t macos \\"
info "    -u \"\$APPLE_ID\" -p \"@keychain:AC_PASSWORD\""
