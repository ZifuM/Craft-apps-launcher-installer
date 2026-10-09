#!/usr/bin/env bash
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo/ArtCraftLauncher-Source"
export MACOSX_DEPLOYMENT_TARGET=12.0
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --locked --release --bin artcraft-launcher --target aarch64-apple-darwin
cargo build --locked --release --bin artcraft-launcher --target x86_64-apple-darwin
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
app="$stage/disk/ArtCraft Master Suite.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$stage/ArtCraft.iconset" "$repo/dist/macos"
lipo -create target/aarch64-apple-darwin/release/artcraft-launcher target/x86_64-apple-darwin/release/artcraft-launcher -output "$app/Contents/MacOS/artcraft-launcher"
chmod 755 "$app/Contents/MacOS/artcraft-launcher"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" assets/artcraft-icon.png --out "$stage/ArtCraft.iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" assets/artcraft-icon.png --out "$stage/ArtCraft.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$stage/ArtCraft.iconset" -o "$app/Contents/Resources/ArtCraft.icns"
python3 "$repo/packaging/macos/write-plist.py" Cargo.toml "$app/Contents/Info.plist"
cp LICENSE "$app/Contents/Resources/LICENSE.txt"
# An ad-hoc signature supports Apple Silicon execution but is not Developer ID
# signing or notarization. Never remove quarantine or disable Gatekeeper.
codesign --force --sign - --timestamp=none "$app"
codesign --verify --deep --strict "$app"
lipo "$app/Contents/MacOS/artcraft-launcher" -verify_arch arm64 x86_64
ln -s /Applications "$stage/disk/Applications"
cp "$repo/packaging/macos/INSTALL.txt" "$stage/disk/INSTALL.txt"
output="$repo/dist/macos/ArtCraftMasterSuite-macOS-universal.dmg"
hdiutil create -volname "ArtCraft Master Suite" -srcfolder "$stage/disk" -ov -format UDZO "$output"
hdiutil verify "$output"
cd "$repo/dist/macos"
shasum -a 256 ArtCraftMasterSuite-macOS-universal.dmg > ArtCraftMasterSuite-macOS-universal.dmg.sha256

