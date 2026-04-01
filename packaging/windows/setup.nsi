!define APPNAME "LiRays-SCADA"
!define COMPANY "LiRays"
; VERSION is injected via -DVERSION=... at build time
; Default fallback if not provided:
!ifndef VERSION
!define VERSION "0.1.0"
!endif
!ifndef ARCH
!define ARCH "x86_64"
!endif
!define INSTALLDIR "$PROGRAMFILES64\\LiRays"
!define DATADIR "C:\\ProgramData\\LiRays-Scada\\data_dir"
!define LOGDIR  "C:\\ProgramData\\LiRays-Scada\\logs"
!define SERVICENAME "LiRaysScada"

OutFile "LiRays-Scada-${VERSION}-${ARCH}-Setup.exe"
InstallDir "${INSTALLDIR}"
InstallDirRegKey HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "InstallLocation"
RequestExecutionLevel admin
SetCompress auto
SetCompressor /SOLID lzma

Section "Install"
  SetOutPath "$INSTDIR"
  File "lirays-scada.exe"
  File "nssm.exe"
  File "install.ps1"
  File "uninstall.ps1"

  CreateDirectory "${DATADIR}"
  CreateDirectory "${LOGDIR}"

  ; Register service with NSSM
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" install ${SERVICENAME} "$INSTDIR\\lirays-scada.exe"'
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" set ${SERVICENAME} AppDirectory "$INSTDIR"'
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" set ${SERVICENAME} Start SERVICE_AUTO_START'
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" set ${SERVICENAME} AppEnvironmentExtra "DATA_DIR=${DATADIR}" "BIND_PORT=8245"'
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" set ${SERVICENAME} AppStdout "${LOGDIR}\\lirays-scada.out.log"'
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" set ${SERVICENAME} AppStderr "${LOGDIR}\\lirays-scada.err.log"'
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" start ${SERVICENAME}'

  WriteUninstaller "$INSTDIR\\Uninstall.exe"
  ; Add uninstall entry
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "DisplayName" "${APPNAME}"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "Publisher" "${COMPANY}"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "UninstallString" "$\"$INSTDIR\\Uninstall.exe$\""
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "QuietUninstallString" "$\"$INSTDIR\\Uninstall.exe$\" /S"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "DisplayIcon" "$INSTDIR\\lirays-scada.exe"
  WriteRegDWORD HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "NoModify" 1
  WriteRegDWORD HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" stop ${SERVICENAME}'
  nsExec::ExecToLog '"$INSTDIR\\nssm.exe" remove ${SERVICENAME} confirm'
  Delete "$INSTDIR\\lirays-scada.exe"
  Delete "$INSTDIR\\nssm.exe"
  Delete "$INSTDIR\\install.ps1"
  Delete "$INSTDIR\\uninstall.ps1"
  Delete "$INSTDIR\\Uninstall.exe"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APPNAME}"
  ; Keep ProgramData by default; user can delete manually if desired
  SetRebootFlag false
SectionEnd
