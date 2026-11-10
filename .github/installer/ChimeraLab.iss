
#define MyAppName "Chimera Lab"
#define MyAppVersion "0.4.0"
#define MyAppPublisher "Chimera Community"

[Setup]
AppId={{A8C31D20-7F21-4E83-9B41-CHIMERALAB040}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}

DefaultDirName={commoncf}\VST3\Chimera Lab.vst3
DisableDirPage=yes
DisableProgramGroupPage=yes

OutputDir=..\..\dist
OutputBaseFilename=Chimera-Lab-Windows-Setup

Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin

Uninstallable=yes

[Files]
Source: "..\..\dist\Chimera Lab.vst3\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Messages]
WelcomeLabel2=This will install Chimera Lab VST3 on your computer.

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True;
end;
