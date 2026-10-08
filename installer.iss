; Inno Setup script - builds CraftLauncher-Setup.exe
; Get Inno Setup (free): https://jrsoftware.org/isinfo.php

#define AppName "Craft Launcher"
#define AppVersion "0.1.0"
#define AppExe "craft-launcher.exe"

[Setup]
AppId={{B7C1E6A2-5D0F-4E7A-9A55-C2A1F0D3C0DE}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Craft Suite
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir=dist
OutputBaseFilename=CraftLauncher-Setup
SetupIconFile=assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Files]
Source: "target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
