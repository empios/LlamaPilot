; Only NSIS writes this marker. MSI and unpackaged executables use manual updates.
!macro NSIS_HOOK_POSTINSTALL
  FileOpen $0 "$INSTDIR\llamapilot-install-kind" w
  FileWrite $0 "nsis-v1"
  FileClose $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\llamapilot-install-kind"
!macroend
