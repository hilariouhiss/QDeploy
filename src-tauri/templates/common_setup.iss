; ============================================================
; QDeploy 内置 Inno Setup 模板
;
; 打包阶段 QDeploy 在工作目录里写两个文件，然后调 ISCC：
;   <应用名>.iss       QDeploy 生成：5 个 #define + 一行 #include
;   common_setup.iss   本文件（QDeploy 从 exe 里写出来）
;
; 本文件只认下面 5 个 define，值全部由 QDeploy 算好、校验过，模板自己不再推导默认值：
;   AppName     应用名（= exe 名去掉 .exe = 暂存目录名）
;   AppGuid     36 位 GUID，不带花括号
;   AppVersion  短版本号；决定安装包文件名
;   SourceDir   暂存目录绝对路径，里面有 exe 和全部要打包的文件
;   OutputDir   安装包输出目录绝对路径
;
; 安装包必须叫 "<AppName> <AppVersion> Installer.exe"。QDeploy 靠这个文件名判断本次
; 是否成功，所以 OutputBaseFilename 不能改。
; ============================================================

#define AppExeName AppName + ".exe"

[Setup]
; ISPP 的 {#AppGuid} 只输出裸 GUID，替换时还会吃掉结尾的 }：这里写三个 { 两个 }，
; 预处理后才是 Inno 要的 AppId={{GUID}（两个 { 是 Inno 的转义，AppId 值仍是 {GUID}）。
AppId={{{#AppGuid}}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Chengdu Chuangkesheng Electronic Technology Co., Ltd.
AppPublisherURL=https://www.cdcks.net/
AppSupportURL=https://www.cdcks.net/
AppUpdatesURL=https://www.cdcks.net/
DefaultDirName={code:GetDefaultDirName}
UninstallDisplayIcon={app}\{#AppExeName}
; 64 位 Windows 与 Windows 11 on Arm；32 位系统不在支持范围
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Inno Setup 7 起安装程序本体也编成 64 位（高熵 ASLR）；6.x 没有这条指令
#if VER >= EncodeVer(7,0,0)
SetupArchitecture=x64
#endif
DisableProgramGroupPage=yes
; 按当前用户安装，不弹 UAC
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename={#AppName} {#AppVersion} Installer
SolidCompression=yes
WizardStyle=modern windows11
UsePreviousAppDir=no
; 只出中文界面，不弹语言选择框
ShowLanguageDialog=no

[Languages]
; Inno Setup 7 起自带官方简体中文；6.x 需自行放入 Languages\ 目录
Name: "cn"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}";

[Files]
; 暂存目录里的一切都进安装目录；MinGW 运行库已由部署阶段拷好，这里没有 VC 运行库要处理
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName} {#AppVersion}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName} {#AppVersion}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Code]
procedure SplitCommandLine(CommandLine: String; var ExePath: String; var Params: String);
var
  P: Integer;
begin
  CommandLine := Trim(CommandLine);

  if Copy(CommandLine, 1, 1) = '"' then
  begin
    Delete(CommandLine, 1, 1);
    P := Pos('"', CommandLine);
    ExePath := Copy(CommandLine, 1, P - 1);
    Params := Trim(Copy(CommandLine, P + 1, MaxInt));
  end
  else
  begin
    P := Pos(' ', CommandLine);
    if P > 0 then
    begin
      ExePath := Copy(CommandLine, 1, P - 1);
      Params := Trim(Copy(CommandLine, P + 1, MaxInt));
    end
    else
    begin
      ExePath := CommandLine;
      Params := '';
    end;
  end;
end;

function TryUninstallSameVersion(RootKey: Integer): Boolean;
var
  SubKey: String;
  InstalledVersion: String;
  QuietUninstallString: String;
  ExePath: String;
  Params: String;
  ResultCode: Integer;
begin
  Result := True;

  SubKey := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{' + '{#AppGuid}' + '}_is1';

  if not RegQueryStringValue(RootKey, SubKey, 'DisplayVersion', InstalledVersion) then
    Exit;

  if InstalledVersion <> '{#AppVersion}' then
    Exit;

  if not RegQueryStringValue(RootKey, SubKey, 'QuietUninstallString', QuietUninstallString) then
  begin
    if not RegQueryStringValue(RootKey, SubKey, 'UninstallString', QuietUninstallString) then
    begin
      MsgBox('检测到已安装相同版本，但未找到卸载程序。安装将停止。', mbError, MB_OK);
      Result := False;
      Exit;
    end;
  end;

  SplitCommandLine(QuietUninstallString, ExePath, Params);

  if Pos('/SILENT', UpperCase(Params)) = 0 then
    Params := Params + ' /VERYSILENT /SUPPRESSMSGBOXES /NORESTART';

  if not Exec(ExePath, Params, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    MsgBox('卸载旧版本失败，安装将停止。', mbError, MB_OK);
    Result := False;
    Exit;
  end;

  if ResultCode <> 0 then
  begin
    MsgBox('卸载旧版本未成功完成，退出码：' + IntToStr(ResultCode) + '。安装将停止。', mbError, MB_OK);
    Result := False;
    Exit;
  end;
end;

function InitializeSetup(): Boolean;
begin
  Result := True;

  { 当前用户安装记录 }
  if not TryUninstallSameVersion(HKCU) then
  begin
    Result := False;
    Exit;
  end;

  { 64 位系统级安装记录 }
  if not TryUninstallSameVersion(HKLM64) then
  begin
    Result := False;
    Exit;
  end;

  { 兼容 32 位注册表视图 }
  if not TryUninstallSameVersion(HKLM) then
  begin
    Result := False;
    Exit;
  end;
end;

function GetDefaultDirName(Param: String): String;
var
  SubKey: String;
  InstallLocation: String;
  ParentDir: String;
  LastSlash: Integer;
begin
  SubKey := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{' + '{#AppGuid}' + '}_is1';

  { 依次尝试 HKCU → HKLM64 → HKLM，查找任意版本的旧安装记录 }
  if not RegQueryStringValue(HKCU, SubKey, 'InstallLocation', InstallLocation) then
    if not RegQueryStringValue(HKLM64, SubKey, 'InstallLocation', InstallLocation) then
      RegQueryStringValue(HKLM, SubKey, 'InstallLocation', InstallLocation);

  if InstallLocation <> '' then
  begin
    { 从旧版本路径中提取父目录：
      例如 C:\Program Files\SATB\v1.1.5 或 C:\Program Files\SATB\v1.1.5\
      均会得到 C:\Program Files\SATB，然后拼接当前版本目录。 }
    StringChangeEx(InstallLocation, '/', '\', True);

    LastSlash := RPos('\', InstallLocation);
    if LastSlash > 0 then
      ParentDir := Copy(InstallLocation, 1, LastSlash - 1)
    else
      ParentDir := InstallLocation;

    Result := ParentDir + '\{#AppVersion}';
  end
    else
    Result := ExpandConstant('{autopf}') + '\{#AppName}\{#AppVersion}';
end;
