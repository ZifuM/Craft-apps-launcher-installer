import hashlib, json, pathlib, sys, urllib.request
arch, destination = sys.argv[1:]
name = f"linuxdeploy-{arch}.AppImage"
request = urllib.request.Request("https://api.github.com/repos/linuxdeploy/linuxdeploy/releases/tags/continuous", headers={"User-Agent": "ArtCraft-Packaging"})
with urllib.request.urlopen(request) as response:
    release = json.load(response)
asset = next(a for a in release["assets"] if a["name"] == name)
digest = (asset.get("digest") or "")
if not digest.startswith("sha256:"):
    raise SystemExit("No published SHA-256 digest. Set LINUXDEPLOY to a separately verified local copy.")
with urllib.request.urlopen(asset["browser_download_url"]) as response:
    data = response.read()
if hashlib.sha256(data).hexdigest() != digest.removeprefix("sha256:"):
    raise SystemExit("linuxdeploy checksum failed")
pathlib.Path(destination).write_bytes(data)
