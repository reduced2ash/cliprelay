#!/bin/zsh
set -euo pipefail

cd "${0:A:h}/.."
uv sync --frozen --extra dev

release_version="${CLIPRELAY_VERSION:-0.1.0}"
export CLIPRELAY_VERSION="$release_version"

ffmpeg_binary="${CLIPRELAY_FFMPEG_DIR:-}/ffmpeg"
ffprobe_binary="${CLIPRELAY_FFMPEG_DIR:-}/ffprobe"
if [[ ! -x "$ffmpeg_binary" ]]; then
  ffmpeg_binary="$(command -v ffmpeg || true)"
fi
if [[ ! -x "$ffprobe_binary" ]]; then
  ffprobe_binary="$(command -v ffprobe || true)"
fi
if [[ "${CLIPRELAY_REQUIRE_FFMPEG:-0}" == "1" ]] \
  && { [[ ! -x "$ffmpeg_binary" ]] || [[ ! -x "$ffprobe_binary" ]]; }; then
  echo "FFmpeg and FFprobe are required for a distributable build." >&2
  exit 1
fi

PYTHONPATH=src uv run python packaging/make_icons.py
PYTHONPATH=src uv run pyinstaller --noconfirm --clean packaging/ClipRelay.spec

# Cloud-backed workspaces can attach Finder metadata that codesign rejects. A
# metadata-free copy gives both ad-hoc development signing and Developer ID
# signing a clean bundle.
clean_root="$(mktemp -d)"
trap 'rm -rf "$clean_root"' EXIT
build_arch="$(uname -m)"
ditto --norsrc --noextattr dist/ClipRelay.app "$clean_root/ClipRelay.app"

sign_identity="${CLIPRELAY_CODESIGN_IDENTITY:--}"
if [[ "$sign_identity" == "-" ]]; then
  codesign --force --deep --sign - "$clean_root/ClipRelay.app"
  signing_label="ad-hoc development signature"
else
  codesign \
    --force \
    --options runtime \
    --timestamp \
    --sign "$sign_identity" \
    --entitlements packaging/macos-entitlements.plist \
    "$clean_root/ClipRelay.app"
  signing_label="$sign_identity"
fi
codesign --verify --deep --strict "$clean_root/ClipRelay.app"

app_notarized="no"
dmg_notarized="no"
if [[ "$sign_identity" != "-" ]] \
  && [[ -n "${APPLE_ID:-}" ]] \
  && [[ -n "${APPLE_APP_PASSWORD:-}" ]] \
  && [[ -n "${APPLE_TEAM_ID:-}" ]]; then
  notarization_zip="$clean_root/ClipRelay-notarization.zip"
  ditto -c -k --norsrc --noextattr --keepParent \
    "$clean_root/ClipRelay.app" "$notarization_zip"
  xcrun notarytool submit "$notarization_zip" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "$clean_root/ClipRelay.app"
  xcrun stapler validate "$clean_root/ClipRelay.app"
  app_notarized="yes"
fi

ditto -c -k --norsrc --noextattr --keepParent \
  "$clean_root/ClipRelay.app" "dist/ClipRelay-macOS-$build_arch.zip"

dmg_root="$clean_root/dmg"
mkdir -p "$dmg_root"
ditto --norsrc --noextattr "$clean_root/ClipRelay.app" "$dmg_root/ClipRelay.app"
ln -s /Applications "$dmg_root/Applications"
hdiutil create \
  -volname "ClipRelay" \
  -srcfolder "$dmg_root" \
  -ov \
  -format UDZO \
  "dist/ClipRelay-macOS-$build_arch.dmg"

if [[ "$app_notarized" == "yes" ]]; then
  codesign \
    --force \
    --timestamp \
    --sign "$sign_identity" \
    "dist/ClipRelay-macOS-$build_arch.dmg"
  codesign --verify --strict "dist/ClipRelay-macOS-$build_arch.dmg"
  xcrun notarytool submit "dist/ClipRelay-macOS-$build_arch.dmg" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "dist/ClipRelay-macOS-$build_arch.dmg"
  xcrun stapler validate "dist/ClipRelay-macOS-$build_arch.dmg"
  dmg_notarized="yes"
fi

{
  echo "ClipRelay $release_version"
  echo "Architecture: $build_arch"
  echo "Application signing: $signing_label"
  echo "Application notarized: $app_notarized"
  echo "Disk image notarized: $dmg_notarized"
  echo
  if [[ -x "$ffmpeg_binary" ]]; then
    "$ffmpeg_binary" -hide_banner -version
    echo
    "$ffmpeg_binary" -hide_banner -L
  else
    echo "FFmpeg was not bundled."
  fi
} >"dist/FFmpeg-build-info-macOS-$build_arch.txt"

# Cloud-backed folders can briefly recreate metadata while PyInstaller output is
# being removed. The signed ZIP above is complete, so stale build directories
# should never turn a successful package into a failed build.
rm -rf dist/ClipRelay.app dist/ClipRelay 2>/dev/null || true

echo "Built macOS ZIP and DMG artifacts for $build_arch"
