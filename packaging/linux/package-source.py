"""Create the single-download Linux source release; no compiled binaries included."""
from pathlib import Path
import gzip
import hashlib
import io
import tarfile
import tomllib

# Avoid resolve(): packaging also runs under Windows filesystem sandboxes.
ROOT = Path(__file__).absolute().parents[2]
VERSION = tomllib.loads((ROOT / "ArtCraftLauncher-Source/Cargo.toml").read_text())["package"]["version"]
BUILD = VERSION.removesuffix(".0")
STEM = f"ArtCraftMasterSuite-{BUILD}-Linux-Source"
ARCHIVE = ROOT / f"{STEM}.tar.gz"
EXCLUDE = {"target", ".git", "backups", "__pycache__", ".pytest_cache", "vendor-registry"}
files = []
for folder in ["ArtCraftLauncher-Source", "packaging/linux", "docs", ".github/workflows"]:
    for path in (ROOT / folder).rglob("*"):
        relative = path.relative_to(ROOT)
        if path.is_file() and not path.is_symlink() and not EXCLUDE.intersection(relative.parts):
            files.append(path)
for name in ["README.md", "RELEASE_NOTES_3.0.md", "RELEASE_NOTES_2.2.md", ".gitignore"]:
    files.append(ROOT / name)

start = f"""# ArtCraft Master Suite - Build {BUILD} Linux Source Preview

This is a source-and-build-files bundle, not an installer.
There is no compiled AppImage or Flatpak in this archive.

1. Extract the complete folder, preserving its structure.
2. Read docs/LINUX.md for Linux prerequisites and platform limitations.
3. From this folder on Linux, choose a build:

   bash packaging/linux/build-appimage.sh
   bash packaging/linux/build-flatpak.sh

Build on the desired CPU architecture (x86-64 or ARM64).
Building requires development tools and internet access for dependencies.
Generated packages appear in dist/linux/.

Read RELEASE_NOTES_3.0.md for this release. README.md describes the full
repository; its Windows binary links are not included in this Linux bundle.
Linux compilation and desktop behavior still need native validation.

Source licenses are retained with the source and vendored dependencies.
SHA256SUMS-SOURCE.txt lists the hashes of all other files in this archive.
""".encode()
manifest = []
partial = ARCHIVE.with_name(ARCHIVE.name + ".partial")
with partial.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
    with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as tar:
        def add(name, data, executable=False):
            info = tarfile.TarInfo(f"{STEM}/{name}")
            info.size = len(data)
            info.mode = 0o755 if executable else 0o644
            info.mtime = 0
            tar.addfile(info, io.BytesIO(data))
            manifest.append(f"{hashlib.sha256(data).hexdigest()}  {name}\n")
        add("START_HERE.md", start)
        for path in sorted(set(files)):
            relative = path.relative_to(ROOT).as_posix()
            add(relative, path.read_bytes(), path.suffix == ".sh")
        add("SHA256SUMS-SOURCE.txt", "".join(manifest).encode())
partial.replace(ARCHIVE)
with ARCHIVE.open("rb") as stream:
    digest = hashlib.file_digest(stream, "sha256").hexdigest()
sums = ROOT / "SHA256SUMS.txt"
lines = sums.read_text().splitlines() if sums.exists() else []
lines = [line for line in lines if not line.endswith("  " + ARCHIVE.name)]
sums.write_text("\n".join(lines + [f"{digest}  {ARCHIVE.name}"]) + "\n", encoding="utf-8")
print(f"Created {ARCHIVE.name}: {ARCHIVE.stat().st_size:,} bytes; {len(files)} source/documentation files")
