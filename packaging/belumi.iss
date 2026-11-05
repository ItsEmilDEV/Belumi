; Inno Setup script for the Belumi launcher and client. Built by packaging\build-release.ps1.
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\dist\belumi"
#endif
#ifndef OutputDir
  #define OutputDir "..\dist"
#endif
#ifndef OutputName
  #define OutputName "belumi-setup"
#endif

[Setup]
AppId={{6F1E9B5A-4C0D-4E7B-9F58-2B1A5E0C7D31}
AppName=Belumi
AppVersion={#AppVersion}
AppPublisher=Belumi
DefaultDirName={autopf}\Belumi
DefaultGroupName=Belumi
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
UninstallDisplayName=Belumi
LicenseFile={#SourceDir}\LICENSE

[Files]
Source: "{#SourceDir}\belumi-launcher.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\horizon-client.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme

[Icons]
Name: "{autoprograms}\Belumi"; Filename: "{app}\belumi-launcher.exe"
Name: "{autodesktop}\Belumi"; Filename: "{app}\belumi-launcher.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Run]
Filename: "{app}\belumi-launcher.exe"; Description: "Start Belumi"; Flags: nowait postinstall skipifsilent
