; The Windows installer, built by the release workflow with Inno Setup:
;   iscc /DAppVersion=0.1.0 /DSourceDir=<folder with the files> poetautopublish.iss
;
; Installs for the current user only (no administrator rights): the program,
; a `poet` command on the PATH, Start menu entries, and an uninstaller.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "."
#endif

[Setup]
AppId={{6F1C2B9E-3D4A-4E5B-9C7D-0A1B2C3D4E5F}
AppName=poetautopublish
AppVersion={#AppVersion}
AppPublisher=D4shwood
AppPublisherURL=https://github.com/D4shwood/poet_autopublish
DefaultDirName={localappdata}\Programs\poetautopublish
DefaultGroupName=poetautopublish
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputBaseFilename=poetautopublish-setup
OutputDir=.
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
UninstallDisplayIcon={app}\poetautopublish.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "tr"; MessagesFile: "compiler:Languages\Turkish.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
tr.NewPoem=Yeni şiir
en.NewPoem=New poem
tr.AddToPath=poet komutunu ekle (komut satırından açmak için)
en.AddToPath=Add the poet command (to start it from a terminal)
tr.RunNow=poetautopublish'i şimdi aç
en.RunNow=Open poetautopublish now

[Tasks]
Name: "path"; Description: "{cm:AddToPath}"

[Files]
Source: "{#SourceDir}\poetautopublish.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\poet.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.tr.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\config.example.toml"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\poetautopublish"; Filename: "{app}\poetautopublish.exe"; WorkingDir: "{userdocs}"
Name: "{group}\{cm:NewPoem}"; Filename: "{app}\poetautopublish.exe"; Parameters: "new"; WorkingDir: "{userdocs}"
Name: "{group}\{cm:UninstallProgram,poetautopublish}"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\poetautopublish.exe"; Description: "{cm:RunNow}"; Flags: nowait postinstall skipifsilent

[Code]
const
  EnvKey = 'Environment';

{ Whether the user's PATH already lists the folder. }
function PathHas(Dir: string): Boolean;
var
  Path: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path) then
    Path := '';
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Path) + ';') > 0;
end;

procedure AddToPath(Dir: string);
var
  Path: string;
begin
  if PathHas(Dir) then
    exit;
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path) then
    Path := '';
  if (Path <> '') and (Copy(Path, Length(Path), 1) <> ';') then
    Path := Path + ';';
  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path + Dir);
end;

procedure RemoveFromPath(Dir: string);
var
  Path: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path) then
    exit;
  Path := ';' + Path + ';';
  P := Pos(';' + Uppercase(Dir) + ';', Uppercase(Path));
  if P = 0 then
    exit;
  Delete(Path, P, Length(Dir) + 1);
  Path := Copy(Path, 2, Length(Path) - 2);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('path') then
    AddToPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
