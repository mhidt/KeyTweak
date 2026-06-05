!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  SetOutPath "$INSTDIR\translator"
  File "${SRCTAURIDIR}\binaries\translator\translator.exe"

  ${If} $TranslatorCheckboxState == ${BST_CHECKED}
    CreateDirectory "$INSTDIR\translator-models"

    DetailPrint "Скачивание модели en→ru..."
    NSISdl::download "https://argos-net.com/v1/translate-en_ru-1_9.argosmodel" "$TEMP\translate-en_ru-1_9.zip"
    Pop $0
    ${If} $0 == "success"
      DetailPrint "Распаковка модели en→ru..."
      nsExec::ExecToStack 'tar -xf "$TEMP\translate-en_ru-1_9.zip" -C "$INSTDIR\translator-models"'
      Pop $1
      Pop $2
      ${If} $1 <> 0
        MessageBox MB_ICONEXCLAMATION|MB_OK "Не удалось распаковать модель перевода en→ru ($2).$\nМодели можно скачать позже из приложения."
      ${EndIf}
      Delete "$TEMP\translate-en_ru-1_9.zip"
    ${Else}
      MessageBox MB_ICONEXCLAMATION|MB_OK "Не удалось скачать модель перевода en→ru ($0).$\nМодели можно скачать позже из приложения."
    ${EndIf}

    DetailPrint "Скачивание модели ru→en..."
    NSISdl::download "https://argos-net.com/v1/translate-ru_en-1_9.argosmodel" "$TEMP\translate-ru_en-1_9.zip"
    Pop $0
    ${If} $0 == "success"
      DetailPrint "Распаковка модели ru→en..."
      nsExec::ExecToStack 'tar -xf "$TEMP\translate-ru_en-1_9.zip" -C "$INSTDIR\translator-models"'
      Pop $1
      Pop $2
      ${If} $1 <> 0
        MessageBox MB_ICONEXCLAMATION|MB_OK "Не удалось распаковать модель перевода ru→en ($2).$\nМодели можно скачать позже из приложения."
      ${EndIf}
      Delete "$TEMP\translate-ru_en-1_9.zip"
    ${Else}
      MessageBox MB_ICONEXCLAMATION|MB_OK "Не удалось скачать модель перевода ru→en ($0).$\nМодели можно скачать позже из приложения."
    ${EndIf}
  ${EndIf}

  SetOutPath "$INSTDIR"
  WriteRegStr HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" "$INSTDIR\${MAINBINARYNAME}.exe"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  FindWindow $0 "" "${PRODUCTNAME}"
  ${If} $0 != 0
    SendMessage $0 ${WM_CLOSE} 0 0
    Sleep 3000
  ${EndIf}
  nsExec::ExecToStack 'taskkill /f /im translator.exe'
  Sleep 1000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  RMDir /r "$INSTDIR\translator"
  RMDir /r "$INSTDIR\translator-models"
!macroend
