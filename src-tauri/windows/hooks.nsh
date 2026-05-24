; NSIS Installer Hooks for RustVerse
; Cleans up runtime-created files during uninstall

!macro NSIS_HOOK_POSTUNINSTALL
  ; Recursively remove entire install directory including all runtime-created files
  ${If} ${FileExists} "$INSTDIR\*.*"
    RMDir /r "$INSTDIR"
  ${EndIf}
!macroend