; Builds from macOS produce NSIS uninstallers whose CRC often fails
; (electron-builder#4875). Disable CRC so uninstall/update can run.
!macro customHeader
  CRCCheck off
!macroend

!macro killGoogleFotoManagerProcesses
  DetailPrint "Chiusura eventuali istanze di Google Foto Manager..."
  nsExec::ExecToLog 'taskkill /F /T /IM "Google Foto Manager.exe"'
  Pop $0
  Pop $1
  nsExec::ExecToLog 'taskkill /F /T /IM "Google Foto Manager Helper.exe"'
  Pop $0
  Pop $1
  nsExec::ExecToLog 'taskkill /F /T /IM "Google Foto Manager Helper (Renderer).exe"'
  Pop $0
  Pop $1
  nsExec::ExecToLog 'taskkill /F /T /IM "Google Foto Manager Helper (GPU).exe"'
  Pop $0
  Pop $1
  nsExec::ExecToLog 'taskkill /F /T /IM "Google Foto Manager Helper (Plugin).exe"'
  Pop $0
  Pop $1
  Sleep 1000
!macroend

!macro preInit
  !insertmacro killGoogleFotoManagerProcesses
!macroend

!macro customUnInstall
  !insertmacro killGoogleFotoManagerProcesses
!macroend

!macro customCheckAppRunning
  !insertmacro killGoogleFotoManagerProcesses
!macroend
