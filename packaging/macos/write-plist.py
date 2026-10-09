import pathlib
import plistlib
import sys
import tomllib

version = tomllib.loads(pathlib.Path(sys.argv[1]).read_text())["package"]["version"]
info = {
    "CFBundleName": "ArtCraft Master Suite",
    "CFBundleDisplayName": "ArtCraft Master Suite",
    "CFBundleIdentifier": "io.github.ZifuM.ArtCraftMasterSuite",
    "CFBundleExecutable": "artcraft-launcher",
    "CFBundleIconFile": "ArtCraft.icns",
    "CFBundlePackageType": "APPL",
    "CFBundleShortVersionString": version,
    "CFBundleVersion": version,
    "LSMinimumSystemVersion": "12.0",
    "NSHighResolutionCapable": True,
    "NSSupportsAutomaticGraphicsSwitching": True,
    "NSPrincipalClass": "NSApplication",
    "NSDocumentsFolderUsageDescription": "Manage projects in the folders you choose.",
    "NSDownloadsFolderUsageDescription": "Install packages and import files you choose.",
    "NSDesktopFolderUsageDescription": "Manage projects in the folders you choose.",
}
pathlib.Path(sys.argv[2]).write_bytes(plistlib.dumps(info))

