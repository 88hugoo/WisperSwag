; Ganchos del instalador NSIS de Wispr Swag (bundle > windows > nsis > installerHooks).

Var WisprSwagExitCode

!macro WISPR_SWAG_CLOSE_RUNNING
  ; Pide a la instancia abierta que se cierre de forma ordenada (libera el
  ; micrófono y el modelo) antes de sustituir o borrar archivos.
  ${If} ${FileExists} "$INSTDIR\wispr-swag.exe"
    nsExec::Exec '"$INSTDIR\wispr-swag.exe" --quit'
    Pop $WisprSwagExitCode
    Sleep 1200
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro WISPR_SWAG_CLOSE_RUNNING
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro WISPR_SWAG_CLOSE_RUNNING
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Quita el inicio automático con Windows para no dejar una entrada huérfana.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Wispr Swag"
  ; Si se marcó «Borrar los datos de la aplicación», se eliminan también los
  ; ajustes, el historial y los modelos (no usan el identificador del paquete).
  ${If} $DeleteAppDataCheckboxState = 1
    RMDir /r "$APPDATA\WisprSwag"
    RMDir /r "$LOCALAPPDATA\WisprSwag"
  ${EndIf}
!macroend
