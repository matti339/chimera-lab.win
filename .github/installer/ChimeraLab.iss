#define MyAppName "Chimera Lab"
#define MyAppVersion "0.4.0-alpha.1"
#define MyAppPublisher "Chimera Community"

[Setup]
AppId={{A8C31D20-7F21-4E83-9B41-04C1A0B04000}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}

DefaultDirName={commoncf64}\VST3
DisableDirPage=yes
DisableProgramGroupPage=yes

ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64

PrivilegesRequired=admin

OutputDir=..\..\dist
OutputBaseFilename=Chimera-Lab-Windows-Setup

Compression=lzma2
SolidCompression=yes
WizardStyle=modern

Uninstallable=yes

[Files]
Source: "..\..\dist\Chimera Lab.vst3"; DestDir: "{app}"; Flags: ignoreversion

[Messages]
WelcomeLabel2=This wizard will install Chimera Lab VST3 on your computer.
