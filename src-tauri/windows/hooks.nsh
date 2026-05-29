; NSIS Installer Hooks for RustVerse
; 确保安装/更新时正确关闭运行中的应用

!include LogicLib.nsh

Var /GLOBAL DeleteUserData_State
Var /GLOBAL DataDirPath

!macro NSIS_HOOK_PREUNINSTALL
  ; 默认数据目录
  StrCpy $DataDirPath "$INSTDIR\data"

  ; 尝试读取 config.toml 中的自定义 data_dir
  ${If} ${FileExists} "$INSTDIR\config.toml"
    ClearErrors
    FileOpen $0 "$INSTDIR\config.toml" r
    ${Do}
      FileRead $0 $1
      ${If} ${Errors}
        ${Break}
      ${EndIf}
      StrCpy $2 $1 9
      ${If} $2 == "data_dir "
        StrCpy $2 $1 10
        ${If} $2 == "data_dir ="
          StrCpy $3 $1 "" 10
          StrCpy $4 $3 1
          ${If} $4 == ' '
            StrCpy $3 $3 "" 1
          ${EndIf}
          StrCpy $4 $3 1
          ${If} $4 == '"'
            StrCpy $3 $3 "" 1
          ${EndIf}
          StrLen $5 $3
          IntOp $5 $5 - 1
          StrCpy $6 $3 1 $5
          ${If} $6 == '"'
            StrCpy $3 $3 $5
          ${EndIf}
          IntOp $5 $5 - 1
          StrCpy $6 $3 1 $5
          ${If} $6 == '$\r'
          ${OrIf} $6 == '$\n'
            StrCpy $3 $3 $5
          ${EndIf}
          StrCpy $7 $3 9
          ${If} $7 == "$$EXE_DIR"
            StrCpy $3 "$INSTDIR$3" "" 9
          ${EndIf}
          StrCpy $DataDirPath $3
          ${Break}
        ${EndIf}
      ${EndIf}
    ${Loop}
    FileClose $0
  ${EndIf}

  DetailPrint "Data directory: $DataDirPath"

  ; 使用 MessageBox 询问是否删除用户数据
  ; MB_YESNO: "是"=删除用户数据, "否"=保留用户数据
  MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 \
    "是否删除用户数据？$\n$\n用户数据包含您的配置文件、缓存和设置。$\n如果您打算重新安装，可以保留这些数据。$\n$\n点击「是」删除用户数据，点击「否」保留用户数据。" \
    IDYES DeleteUserData IDNO PreserveUserData

  DeleteUserData:
    StrCpy $DeleteUserData_State 1
    Goto DoneAsk

  PreserveUserData:
    StrCpy $DeleteUserData_State 0

  DoneAsk:
    DetailPrint "Delete user data: $DeleteUserData_State"
!macroend

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Closing RustVerse..."

  ExecWait 'taskkill /IM RustVerse.exe /T /F 2>nul'

  Sleep 2000

  DetailPrint "RustVerse closed."
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Sleep 500
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "Cleaning up application files..."

  ; 删除应用程序可执行文件
  ${If} ${FileExists} "$INSTDIR\RustVerse.exe"
    Delete "$INSTDIR\RustVerse.exe"
  ${EndIf}

  ; 删除 Tauri 相关 DLL 文件
  ${If} ${FileExists} "$INSTDIR\*.dll"
    Delete "$INSTDIR\*.dll"
  ${EndIf}

  ; 删除其他应用程序文件
  ${If} ${FileExists} "$INSTDIR\*.pdb"
    Delete "$INSTDIR\*.pdb"
  ${EndIf}

  ${If} ${FileExists} "$INSTDIR\*.exe"
    Delete "$INSTDIR\*.exe"
  ${EndIf}

  ; 删除 resources 目录（不包含用户数据）
  ${If} ${FileExists} "$INSTDIR\resources\*.*"
    RMDir /r "$INSTDIR\resources"
  ${EndIf}

  ; 删除 locales 目录
  ${If} ${FileExists} "$INSTDIR\locales\*.*"
    RMDir /r "$INSTDIR\locales"
  ${EndIf}

  ; 删除 webview 目录（WebView2 缓存，非用户数据）
  ${If} ${FileExists} "$INSTDIR\webview\*.*"
    RMDir /r "$INSTDIR\webview"
  ${EndIf}

  ; 删除日志目录
  ${If} ${FileExists} "$INSTDIR\logs\*.*"
    RMDir /r "$INSTDIR\logs"
  ${EndIf}

  ; 根据用户选择决定是否删除用户数据目录
  ${If} $DeleteUserData_State == 1
    DetailPrint "Deleting user data directory: $DataDirPath"
    ${If} ${FileExists} "$DataDirPath\*.*"
      RMDir /r "$DataDirPath"
    ${EndIf}
  ${Else}
    DetailPrint "Preserving user data directory: $DataDirPath"
  ${EndIf}

  ; 尝试删除空的安装目录
  RMDir "$INSTDIR"

  DetailPrint "Application files cleaned up."
!macroend
