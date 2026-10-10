#!/usr/bin/env bash
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
arch="$(uname -m)"
case "$arch" in x86_64|aarch64) ;; *) echo "Unsupported architecture: $arch" >&2; exit 1;; esac
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/source"
python3 - "$repo/ArtCraftLauncher-Source" "$stage/source" <<'PY'
import shutil,sys
shutil.copytree(sys.argv[1],sys.argv[2],dirs_exist_ok=True,ignore=shutil.ignore_patterns('target','.git','vendor-registry'))
PY
cp "$repo/packaging/linux/"*.desktop "$repo/packaging/linux/"*.xml "$stage/source/"
cp "$repo/packaging/linux/io.github.ZifuM.ArtCraftMasterSuite.json" "$stage/manifest.json"
# Cargo runs inside the Flatpak build sandbox and does not inherit the host's
# OAuth build variable. Forward it through the temporary, untracked manifest.
python3 - "$stage/manifest.json" <<'PY'
import json, os, pathlib, sys
path = pathlib.Path(sys.argv[1])
manifest = json.loads(path.read_text())
secret = os.environ.get('ARTCRAFT_GOOGLE_DESKTOP_CLIENT_SECRET', '')
if secret:
    manifest.setdefault('build-options', {}).setdefault('env', {})['ARTCRAFT_GOOGLE_DESKTOP_CLIENT_SECRET'] = secret
path.write_text(json.dumps(manifest, indent=2))
PY
cd "$stage/source"
mkdir -p .cargo
cargo vendor --locked vendor-registry > .cargo/config.toml
python3 - <<'PY'
from pathlib import Path
p=Path('.cargo/config.toml');s=p.read_text();s=s.replace(str(Path('vendor-registry').absolute()),'vendor-registry');p.write_text(s)
PY
mkdir -p "$repo/dist/linux"
flatpak-builder --user --install-deps-from=flathub --repo="$repo/dist/linux/flatpak-repo" "$stage/build" "$stage/manifest.json"
flatpak build-bundle --arch="$arch" --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo "$repo/dist/linux/flatpak-repo" "$repo/dist/linux/Linux-$arch.flatpak" io.github.ZifuM.ArtCraftMasterSuite
cd "$repo/dist/linux"
sha256sum "Linux-$arch.flatpak" > "Linux-$arch.flatpak.sha256"
