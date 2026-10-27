; Aggiornamento: pulisce residui (profilo WebView, installazioni vecchie, chiavi
; orfane, file sparsi). Non tocca il database locale né le credenziali Supabase.
; Disinstallazione vera: cancella anche quelli.
; Nomi fissi: productName, identifier, binario. Se cambiano, aggiornare qui.
; ${PRODUCTNAME} non è ancora definito quando questo file viene incluso.

!macro AAC_WIPE_WEBVIEW
  RMDir /r "$LOCALAPPDATA\com.ilpano.android-adware-cleaner"
  RMDir /r "$APPDATA\com.ilpano.android-adware-cleaner"
!macroend

!macro AAC_UNQUOTE VAR
  StrCpy $4 ${VAR} 1
  ${If} $4 == "$\""
    StrCpy ${VAR} ${VAR} "" 1
    StrCpy ${VAR} ${VAR} -1
  ${EndIf}
!macroend

!macro AAC_REMOVE_FOREIGN
  StrCpy $R9 "$PROGRAMFILES64\AndroidAdwareCleaner"
  !insertmacro AAC_REMOVE_ONE
  StrCpy $R9 "$PROGRAMFILES\AndroidAdwareCleaner"
  !insertmacro AAC_REMOVE_ONE
  StrCpy $R9 "$PROGRAMFILES32\AndroidAdwareCleaner"
  !insertmacro AAC_REMOVE_ONE
!macroend

!macro AAC_REMOVE_ONE
  ${If} $R9 != ""
  ${AndIf} $R9 != $INSTDIR
  ${AndIf} $R9 != "$INSTDIR\"
  ${AndIf} ${FileExists} "$R9\android-adware-cleaner.exe"
    RMDir /r "$R9"
  ${EndIf}
!macroend

; Chiavi di disinstallazione con lo stesso nome ma cartella assente o diversa.
!macro AAC_DROP_HIVE HIVE ID
  StrCpy $0 0
  aac_${ID}_loop:
    EnumRegKey $1 ${HIVE} "Software\Microsoft\Windows\CurrentVersion\Uninstall" $0
    StrCmp $1 "" aac_${ID}_done
    ReadRegStr $2 ${HIVE} "Software\Microsoft\Windows\CurrentVersion\Uninstall\$1" "DisplayName"
    StrCmp $2 "AndroidAdwareCleaner" 0 aac_${ID}_next
    ReadRegStr $3 ${HIVE} "Software\Microsoft\Windows\CurrentVersion\Uninstall\$1" "InstallLocation"
    !insertmacro AAC_UNQUOTE $3
    StrCmp $3 "$INSTDIR" aac_${ID}_next
    StrCmp $3 "$INSTDIR\" aac_${ID}_next
    ClearErrors
    DeleteRegKey ${HIVE} "Software\Microsoft\Windows\CurrentVersion\Uninstall\$1"
    IfErrors aac_${ID}_next
    Goto aac_${ID}_loop
  aac_${ID}_next:
    IntOp $0 $0 + 1
    Goto aac_${ID}_loop
  aac_${ID}_done:
!macroend

; File sparsi nella cartella dati. Il database e le credenziali restano.
!macro AAC_PRUNE_APPDATA
  ClearErrors
  FindFirst $0 $1 "$APPDATA\AndroidAdwareCleaner\*"
  IfErrors aac_prune_none
  aac_prune_loop:
    StrCmp $1 "" aac_prune_done
    StrCmp $1 "." aac_prune_next
    StrCmp $1 ".." aac_prune_next
    StrCmp $1 "config.json" aac_prune_next
    StrCmp $1 "supabase-session.json" aac_prune_next
    StrCmp $1 "reputation.db" aac_prune_next
    StrCmp $1 "reputation.db-wal" aac_prune_next
    StrCmp $1 "reputation.db-shm" aac_prune_next
    StrCmp $1 "platform-tools" aac_prune_next
    Delete "$APPDATA\AndroidAdwareCleaner\$1"
    RMDir /r "$APPDATA\AndroidAdwareCleaner\$1"
  aac_prune_next:
    FindNext $0 $1
    Goto aac_prune_loop
  aac_prune_done:
    FindClose $0
  aac_prune_none:
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro AAC_WIPE_WEBVIEW
  !insertmacro AAC_REMOVE_FOREIGN
  !insertmacro AAC_DROP_HIVE HKCU updcu
  !insertmacro AAC_DROP_HIVE HKLM updlm
  !insertmacro AAC_PRUNE_APPDATA
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    SetShellVarContext current
    ; nsExec non apre una finestra di console.
    nsExec::Exec '$SYSDIR\taskkill.exe /F /IM adb.exe'
    Pop $R8
    ClearErrors
    RMDir /r "$APPDATA\AndroidAdwareCleaner"
    !insertmacro AAC_WIPE_WEBVIEW
    !insertmacro AAC_REMOVE_FOREIGN
    !insertmacro AAC_DROP_HIVE HKCU uncu
    !insertmacro AAC_DROP_HIVE HKLM unlm
    DeleteRegKey HKCU "Software\ilpano\AndroidAdwareCleaner"
    DeleteRegKey /ifempty HKCU "Software\ilpano"
    DeleteRegKey HKLM "Software\ilpano\AndroidAdwareCleaner"
    DeleteRegKey /ifempty HKLM "Software\ilpano"
  ${EndIf}
!macroend
