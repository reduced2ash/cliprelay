#!/bin/zsh
set -euo pipefail

repo_root="${0:A:h}/.."
cd "$repo_root"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The macOS bundle must be built on macOS." >&2
  exit 1
fi

for command in cargo pkg-config ditto otool install_name_tool codesign hdiutil lipo; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "Missing required build command: $command" >&2
    exit 1
  fi
done

release_version="${CLIPRELAY_VERSION:-$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)}"
build_arch="$(uname -m)"
case "$build_arch" in
  arm64|x86_64) ;;
  *)
    echo "Unsupported macOS architecture: $build_arch" >&2
    exit 1
    ;;
esac

gst_framework="${CLIPRELAY_GSTREAMER_FRAMEWORK:-/Library/Frameworks/GStreamer.framework}"
if [[ ! -d "$gst_framework/Versions/Current" ]]; then
  echo "GStreamer.framework was not found at $gst_framework" >&2
  echo "Install the official GStreamer runtime and development SDK, or set CLIPRELAY_GSTREAMER_FRAMEWORK." >&2
  exit 1
fi
gst_root="$(cd "$gst_framework/Versions/Current" && pwd -P)"
if [[ ! -f "$gst_root/lib/pkgconfig/gstreamer-1.0.pc" ]]; then
  echo "The GStreamer development SDK is missing from $gst_root" >&2
  exit 1
fi
if [[ ! -x "$gst_root/libexec/gstreamer-1.0/gst-plugin-scanner" ]]; then
  echo "The GStreamer plugin scanner is missing from $gst_root" >&2
  exit 1
fi

export PATH="$gst_root/bin:$PATH"
export PKG_CONFIG_PATH="$gst_root/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
pkg-config --atleast-version=1.22 gstreamer-1.0
pkg-config --exists gstreamer-app-1.0 gstreamer-video-1.0

ffmpeg_binary="${CLIPRELAY_FFMPEG_DIR:-}/ffmpeg"
ffprobe_binary="${CLIPRELAY_FFMPEG_DIR:-}/ffprobe"
if [[ ! -x "$ffmpeg_binary" ]]; then
  ffmpeg_binary="$(command -v ffmpeg || true)"
fi
if [[ ! -x "$ffprobe_binary" ]]; then
  ffprobe_binary="$(command -v ffprobe || true)"
fi
if [[ ! -x "$ffmpeg_binary" || ! -x "$ffprobe_binary" ]]; then
  echo "Self-contained packages require FFmpeg and FFprobe." >&2
  echo "Set CLIPRELAY_FFMPEG_DIR to distributable binaries for $build_arch." >&2
  exit 1
fi

verify_portable_tool() {
  local binary="$1"
  local dependency
  while IFS= read -r dependency; do
    case "$dependency" in
      @*|/System/Library/*|/usr/lib/*) ;;
      *)
        echo "$binary depends on an external library: $dependency" >&2
        echo "Use a distributable FFmpeg build instead of a Homebrew-linked binary." >&2
        return 1
        ;;
    esac
  done < <(otool -L "$binary" | awk 'NR > 1 { print $1 }')
}
verify_portable_tool "$ffmpeg_binary"
verify_portable_tool "$ffprobe_binary"

cargo build --release --locked -p cliprelay

app="dist/ClipRelay.app"
contents="$app/Contents"
macos="$contents/MacOS"
frameworks="$contents/Frameworks"
resources="$contents/Resources"
rm -rf "$app"
mkdir -p "$macos/bin" "$frameworks" "$resources/licenses"

ditto --norsrc --noextattr target/release/cliprelay "$macos/ClipRelay"
ditto --norsrc --noextattr "$ffmpeg_binary" "$macos/bin/ffmpeg"
ditto --norsrc --noextattr "$ffprobe_binary" "$macos/bin/ffprobe"
ditto --norsrc --noextattr "$gst_framework" "$frameworks/GStreamer.framework"
ditto --norsrc --noextattr packaging/icons/cliprelay.icns "$resources/cliprelay.icns"
ditto --norsrc --noextattr LICENSE "$resources/licenses/ClipRelay-MIT.txt"
ditto --norsrc --noextattr THIRD_PARTY_NOTICES.md "$resources/THIRD_PARTY_NOTICES.md"
ditto --norsrc --noextattr LICENSES "$resources/licenses"

cat >"$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>ClipRelay</string>
  <key>CFBundleExecutable</key><string>ClipRelay</string>
  <key>CFBundleIconFile</key><string>cliprelay</string>
  <key>CFBundleIdentifier</key><string>app.cliprelay.desktop</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>ClipRelay</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$release_version</string>
  <key>CFBundleVersion</key><string>$release_version</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSRequiresAquaSystemAppearance</key><false/>
</dict>
</plist>
PLIST

embedded_gst_rpath="@executable_path/../Frameworks/GStreamer.framework/Versions/Current/lib"
if ! otool -l "$macos/ClipRelay" | grep -F -q "$embedded_gst_rpath"; then
  install_name_tool -add_rpath "$embedded_gst_rpath" "$macos/ClipRelay"
fi

# The official 1.22+ SDK is relocatable. Normalize direct SDK references in the
# app binary, then reject any remaining host-only references in runtime files.
while IFS= read -r dependency; do
  case "$dependency" in
    "$gst_framework"/Versions/*/lib/*|"$gst_root"/lib/*)
      install_name_tool -change "$dependency" "@rpath/${dependency:t}" "$macos/ClipRelay"
      ;;
  esac
done < <(otool -L "$macos/ClipRelay" | awk 'NR > 1 { print $1 }')

verify_macho_dependencies() {
  local binary="$1"
  local dependency
  while IFS= read -r dependency; do
    case "$dependency" in
      @*|/System/Library/*|/usr/lib/*) ;;
      *)
        echo "Unbundled dependency in $binary: $dependency" >&2
        return 1
        ;;
    esac
  done < <(otool -L "$binary" | awk 'NR > 1 { print $1 }')
}

verify_macho_dependencies "$macos/ClipRelay"
while IFS= read -r -d '' candidate; do
  if file "$candidate" | grep -q 'Mach-O'; then
    verify_macho_dependencies "$candidate"
  fi
done < <(find -H "$frameworks/GStreamer.framework/Versions/Current/lib" \
  "$frameworks/GStreamer.framework/Versions/Current/libexec" -type f -print0)

if ! lipo -archs "$macos/ClipRelay" | tr ' ' '\n' | grep -F -q "$build_arch"; then
  echo "ClipRelay binary does not contain the $build_arch architecture." >&2
  exit 1
fi
if ! lipo -archs "$frameworks/GStreamer.framework/Versions/Current/lib/libgstreamer-1.0.0.dylib" \
  | tr ' ' '\n' | grep -F -q "$build_arch"; then
  echo "Bundled GStreamer does not contain the $build_arch architecture." >&2
  exit 1
fi

sign_identity="${CLIPRELAY_CODESIGN_IDENTITY:--}"
sign_nested() {
  local target="$1"
  if [[ "$sign_identity" == "-" ]]; then
    codesign --force --sign - "$target"
  else
    codesign --force --options runtime --timestamp --sign "$sign_identity" "$target"
  fi
}

while IFS= read -r -d '' candidate; do
  if file "$candidate" | grep -q 'Mach-O'; then
    sign_nested "$candidate"
  fi
done < <(find "$frameworks" "$macos/bin" -type f -print0)
sign_nested "$frameworks/GStreamer.framework"
sign_nested "$macos/ClipRelay"
if [[ "$sign_identity" == "-" ]]; then
  codesign --force --sign - --entitlements packaging/macos-entitlements.plist "$app"
  signing_label="ad-hoc development signature"
else
  codesign --force --options runtime --timestamp --sign "$sign_identity" \
    --entitlements packaging/macos-entitlements.plist "$app"
  signing_label="$sign_identity"
fi
codesign --verify --deep --strict "$app"

mkdir -p dist
{
  echo "ClipRelay $release_version"
  echo "Architecture: $build_arch"
  echo "GStreamer: $(pkg-config --modversion gstreamer-1.0)"
  echo "GStreamer framework: embedded privately"
  echo "Application signing: $signing_label"
  echo
  "$ffmpeg_binary" -hide_banner -version
  echo
  "$ffmpeg_binary" -hide_banner -L
} >"dist/Media-build-info-macOS-$build_arch.txt"

app_notarized="no"
dmg_notarized="no"
if [[ "$sign_identity" != "-" \
  && -n "${APPLE_ID:-}" \
  && -n "${APPLE_APP_PASSWORD:-}" \
  && -n "${APPLE_TEAM_ID:-}" ]]; then
  notarization_zip="dist/ClipRelay-notarization.zip"
  ditto -c -k --norsrc --noextattr --keepParent "$app" "$notarization_zip"
  xcrun notarytool submit "$notarization_zip" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "$app"
  xcrun stapler validate "$app"
  rm -f "$notarization_zip"
  app_notarized="yes"
fi

zip_path="dist/ClipRelay-macOS-$build_arch.zip"
dmg_path="dist/ClipRelay-macOS-$build_arch.dmg"
rm -f "$zip_path" "$dmg_path"
ditto -c -k --norsrc --noextattr --keepParent "$app" "$zip_path"

dmg_root="$(mktemp -d)"
trap 'rm -rf "$dmg_root"' EXIT
ditto --norsrc --noextattr "$app" "$dmg_root/ClipRelay.app"
ln -s /Applications "$dmg_root/Applications"
hdiutil create -volname ClipRelay -srcfolder "$dmg_root" -ov -format UDZO "$dmg_path"

if [[ "$app_notarized" == "yes" ]]; then
  codesign --force --timestamp --sign "$sign_identity" "$dmg_path"
  xcrun notarytool submit "$dmg_path" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "$dmg_path"
  xcrun stapler validate "$dmg_path"
  dmg_notarized="yes"
fi

{
  echo "Application notarized: $app_notarized"
  echo "Disk image notarized: $dmg_notarized"
} >>"dist/Media-build-info-macOS-$build_arch.txt"

echo "Built self-contained $zip_path and $dmg_path"
