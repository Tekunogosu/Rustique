@echo on
echo Waiting for application to close...
:wait_loop
tasklist /FI "IMAGENAME eq {EXE_NAME}" 2>NUL | find /I "{EXE_NAME}" >NUL
if "%ERRORLEVEL%"=="0" (
    timeout /t 2 /nobreak >NUL
    goto wait_loop
)

echo Creating backup...
copy "{CURRENT_EXE}" "{BACKUP_PATH}" >NUL
if errorlevel 1 (
    echo Failed to create backup
    pause
    exit /b 1
)

echo Replacing binary...
copy "{NEW_BINARY}" "{CURRENT_EXE}" >NUL
if errorlevel 1 (
    echo Failed to replace binary, restoring backup...
    copy "{BACKUP_PATH}" "{CURRENT_EXE}" >NUL
    pause
    exit /b 1
)

echo Starting updated application...
start "" "{CURRENT_EXE} -V"

echo Cleaning up...
del "{BACKUP_PATH}" >NUL 2>&1
del "{NEW_BINARY}" >NUL 2>&1
del "%~f0" >NUL 2>&1