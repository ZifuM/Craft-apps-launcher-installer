#!/usr/bin/env bash
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
arch="$(uname -m)"
case "$arch" in x86_64|aarch64) ;; *) echo "Unsupported architecture: $arch" >&2; exit 1;; esac
cd "$repo/ArtCraftLauncher-Source"
cargo build --locked --release --bin artcraft-launcher
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
appdir="$stage/AppDir"
install -Dm755 target/release/artcraft-launcher "$appdir/usr/bin/artcraft-launcher"
install -Dm644 assets/artcraft-icon.png "$appdir/usr/share/icons/hicolor/256x256/apps/io.github.ZifuM.ArtCraftMasterSuite.png"
install -Dm644 "$repo/packaging/linux/io.github.ZifuM.ArtCraftMasterSuite.desktop" "$appdir/usr/share/applications/io.github.ZifuM.ArtCraftMasterSuite.desktop"
install -Dm644 "$repo/packaging/linux/io.github.ZifuM.ArtCraftMasterSuite.metainfo.xml" "$appdir/usr/share/metainfo/io.github.ZifuM.ArtCraftMasterSuite.metainfo.xml"
if [ -n "${LINUXDEPLOY:-}" ]; then
  deploy="$LINUXDEPLOY"
else
  deploy="$stage/linuxdeploy.AppImage"
  python3 "$repo/packaging/linux/fetch-linuxdeploy.py" "$arch" "$deploy"
  chmod +x "$deploy"
fi
mkdir -p "$repo/dist/linux"
cd "$repo/dist/linux"
export APPIMAGE_EXTRACT_AND_RUN=1
export OUTPUT="Linux-$arch.AppImage"
"$deploy" --appdir "$appdir" --executable "$(command -v pdftoppm)" --output appimage
sha256sum "$OUTPUT" > "$OUTPUT.sha256"
echo "Created $repo/dist/linux/$OUTPUT"
