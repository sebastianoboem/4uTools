!macro NSIS_HOOK_POSTINSTALL
  ; Ricrea i collegamenti già presenti, così Windows prende l'icona nuova.
  IfFileExists "$DESKTOP\4uTools.lnk" 0 desktop_dp
    Delete "$DESKTOP\4uTools.lnk"
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
  desktop_dp:
  IfFileExists "$DESKTOP\${PRODUCTNAME}.lnk" 0 menu_vecchio
    Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
  menu_vecchio:
  IfFileExists "$SMPROGRAMS\4uTools.lnk" 0 menu_dp
    Delete "$SMPROGRAMS\4uTools.lnk"
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
  menu_dp:
  IfFileExists "$SMPROGRAMS\${PRODUCTNAME}.lnk" 0 icona_aggiornata
    Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0
  icona_aggiornata:
  System::Call 'shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
!macroend
