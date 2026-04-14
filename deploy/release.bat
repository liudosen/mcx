@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
set "BACKEND_DIR=%SCRIPT_DIR%..\mcx-system"

pushd "%BACKEND_DIR%" || exit /b 1
call :main %*
set "EXIT_CODE=%ERRORLEVEL%"
popd
exit /b %EXIT_CODE%

:main
set "APP_NAME=mcx-system"
set "DOCKER_IMAGE=mcx-system-builder"
set "PACKAGE_NAME=%APP_NAME%-linux-x86_64"
set "PACKAGE_DIR=release\%PACKAGE_NAME%"
set "PACKAGE_FILE=release\%PACKAGE_NAME%.tar.gz"
set "MERGED_ENV_FILE=release\%PACKAGE_NAME%.env"
set "REMOTE_PACKAGE=%APP_NAME%-linux-x86_64.tar.gz"

if not defined DEPLOY_SERVER set "DEPLOY_SERVER=47.103.220.84"
if not defined DEPLOY_USER set "DEPLOY_USER=root"
if not defined DEPLOY_DIR set "DEPLOY_DIR=/root/workspace/mcx"
if not defined DEPLOY_PORT set "DEPLOY_PORT=8081"
if not defined REMOTE_SYSTEM_ENV set "REMOTE_SYSTEM_ENV=/etc/mcx-system.env"

set "BUILD_WINDOWS="
set "BUILD_LINUX="
set "DEPLOY_REMOTE="
set "CLEAN_BUILD="

:parse_args
if "%~1"=="" goto args_done
if /i "%~1"=="--windows" (
    set "BUILD_WINDOWS=1"
    set "BUILD_LINUX="
    set "DEPLOY_REMOTE="
    shift
    goto parse_args
)
if /i "%~1"=="--linux" (
    set "BUILD_LINUX=1"
    set "BUILD_WINDOWS="
    set "DEPLOY_REMOTE="
    shift
    goto parse_args
)
if /i "%~1"=="--deploy" (
    set "DEPLOY_REMOTE=1"
    set "BUILD_LINUX=1"
    set "BUILD_WINDOWS="
    shift
    goto parse_args
)
if /i "%~1"=="--clean" (
    set "CLEAN_BUILD=1"
    shift
    goto parse_args
)
echo Unknown argument: %~1
echo Usage: %~nx0 [--windows^|--linux^|--deploy] [--clean]
exit /b 1

:args_done
if not defined BUILD_WINDOWS if not defined BUILD_LINUX (
    set "BUILD_LINUX=1"
    set "DEPLOY_REMOTE=1"
)
if defined DEPLOY_REMOTE set "BUILD_LINUX=1"

if defined DEPLOY_REMOTE (
    call :prepare_merged_env || exit /b 1
)

echo ==========================================
echo   mcx-system Release Builder
echo ==========================================
if defined BUILD_WINDOWS (
    echo   Target: Windows x86_64
) else (
    echo   Target: Linux x86_64
    if defined DEPLOY_REMOTE (
        echo   Mode: Build, package, write env, upload, restart
    ) else (
        echo   Mode: Build and package
    )
)
echo ==========================================

if defined CLEAN_BUILD (
    echo.
    echo [clean] Removing previous build outputs...
    if exist "release" rmdir /s /q "release"
    cargo clean
    if errorlevel 1 (
        echo Cleanup failed!
        call :maybe_pause
        exit /b 1
    )
)

if defined BUILD_WINDOWS goto build_windows
goto build_linux

:build_windows
echo.
echo [1/1] Building Windows release...
cargo build --release
if errorlevel 1 (
    echo Build failed!
    call :maybe_pause
    exit /b 1
)
echo.
echo Output: target\release\%APP_NAME%.exe
goto end_success

:build_linux
echo.
echo [1/4] Checking Docker...
where docker >nul 2>nul
if errorlevel 1 (
    echo ERROR: Docker not found. Please install Docker Desktop.
    call :maybe_pause
    exit /b 1
)

docker info >nul 2>nul
if errorlevel 1 (
    echo ERROR: Docker is not running. Please start Docker Desktop.
    call :maybe_pause
    exit /b 1
)

echo.
echo [2/4] Building Docker image...
docker build -f Dockerfile.linux-build -t %DOCKER_IMAGE% .
if errorlevel 1 (
    echo Docker build failed!
    call :maybe_pause
    exit /b 1
)

echo.
echo [3/4] Extracting Linux binary...
if not exist "target" mkdir "target"
set "CONTAINER_NAME=mcx-build-%RANDOM%"
docker create --name %CONTAINER_NAME% %DOCKER_IMAGE% >nul
if errorlevel 1 (
    echo Failed to create Docker container!
    call :maybe_pause
    exit /b 1
)
docker cp %CONTAINER_NAME%:/app/%APP_NAME% "target\%APP_NAME%"
set "COPY_STATUS=%errorlevel%"
docker rm %CONTAINER_NAME% >nul 2>nul
if not "%COPY_STATUS%"=="0" (
    echo Failed to extract Linux binary!
    call :maybe_pause
    exit /b 1
)

echo.
echo [4/4] Packaging release tarball...
where tar >nul 2>nul
if errorlevel 1 (
    echo ERROR: tar command not found.
    call :maybe_pause
    exit /b 1
)
if exist "%PACKAGE_DIR%" rmdir /s /q "%PACKAGE_DIR%"
if exist "%PACKAGE_FILE%" del /q "%PACKAGE_FILE%"
mkdir "%PACKAGE_DIR%"
copy /y "target\%APP_NAME%" "%PACKAGE_DIR%\%APP_NAME%" >nul
tar -czf "%PACKAGE_FILE%" -C "%PACKAGE_DIR%" "%APP_NAME%"
if errorlevel 1 (
    echo Packaging failed!
    call :maybe_pause
    exit /b 1
)

echo.
echo Output:
echo   - target\%APP_NAME%
echo   - %PACKAGE_FILE%

if not defined DEPLOY_REMOTE goto end_success

echo.
echo [deploy] Running password-based remote deployment...
python "%SCRIPT_DIR%deploy_password.py"
if errorlevel 1 (
    echo Remote deployment failed!
    call :maybe_pause
    exit /b 1
)

goto end_success

:prepare_merged_env
if not exist "release" mkdir "release"
set "ENV_RENDER_PS=%TEMP%\mcx-render-env-%RANDOM%.ps1"
> "%ENV_RENDER_PS%" echo $ErrorActionPreference = 'Stop'
>> "%ENV_RENDER_PS%" echo $dst = '%MERGED_ENV_FILE%'
>> "%ENV_RENDER_PS%" echo $envMap = @{}
>> "%ENV_RENDER_PS%" echo $orderedKeys = @(
>> "%ENV_RENDER_PS%" echo   'DATABASE_HOST',
>> "%ENV_RENDER_PS%" echo   'DATABASE_PORT',
>> "%ENV_RENDER_PS%" echo   'DATABASE_NAME',
>> "%ENV_RENDER_PS%" echo   'DATABASE_USER',
>> "%ENV_RENDER_PS%" echo   'DATABASE_PASSWORD',
>> "%ENV_RENDER_PS%" echo   'REDIS_HOST',
>> "%ENV_RENDER_PS%" echo   'REDIS_PORT',
>> "%ENV_RENDER_PS%" echo   'REDIS_USERNAME',
>> "%ENV_RENDER_PS%" echo   'REDIS_PASSWORD',
>> "%ENV_RENDER_PS%" echo   'REDIS_DB',
>> "%ENV_RENDER_PS%" echo   'JWT_SECRET',
>> "%ENV_RENDER_PS%" echo   'JWT_EXPIRY_HOURS',
>> "%ENV_RENDER_PS%" echo   'SERVER_HOST',
>> "%ENV_RENDER_PS%" echo   'SERVER_PORT',
>> "%ENV_RENDER_PS%" echo   'BCRYPT_COST',
>> "%ENV_RENDER_PS%" echo   'ADMIN_USERNAME',
>> "%ENV_RENDER_PS%" echo   'ADMIN_PASSWORD',
>> "%ENV_RENDER_PS%" echo   'WEIXIN_APPID',
>> "%ENV_RENDER_PS%" echo   'WEIXIN_SECRET',
>> "%ENV_RENDER_PS%" echo   'JK_SELLER_USERNAME',
>> "%ENV_RENDER_PS%" echo   'JK_SELLER_PASSWORD',
>> "%ENV_RENDER_PS%" echo   'OSS_ENDPOINT',
>> "%ENV_RENDER_PS%" echo   'OSS_ACCESS_KEY_ID',
>> "%ENV_RENDER_PS%" echo   'OSS_ACCESS_KEY_SECRET',
>> "%ENV_RENDER_PS%" echo   'OSS_BUCKET',
>> "%ENV_RENDER_PS%" echo   'OSS_DOMAIN',
>> "%ENV_RENDER_PS%" echo   'LOG_DIR',
>> "%ENV_RENDER_PS%" echo   'LOG_MAX_FILE_SIZE',
>> "%ENV_RENDER_PS%" echo   'LOG_MAX_AGE_DAYS'
>> "%ENV_RENDER_PS%" echo )
>> "%ENV_RENDER_PS%" echo function Get-EnvValue ^($name^) {
>> "%ENV_RENDER_PS%" echo   foreach ^($scope in @('Process', 'User', 'Machine'^)^) {
>> "%ENV_RENDER_PS%" echo     $value = [Environment]::GetEnvironmentVariable^($name, $scope^)
>> "%ENV_RENDER_PS%" echo     if ^(-not [string]::IsNullOrWhiteSpace^($value^)^) { return $value }
>> "%ENV_RENDER_PS%" echo   }
>> "%ENV_RENDER_PS%" echo   return $null
>> "%ENV_RENDER_PS%" echo }
>> "%ENV_RENDER_PS%" echo foreach ^($key in $orderedKeys^) {
>> "%ENV_RENDER_PS%" echo   $value = Get-EnvValue $key
>> "%ENV_RENDER_PS%" echo   if ^([string]::IsNullOrWhiteSpace^($value^)^) { $value = '' }
>> "%ENV_RENDER_PS%" echo   $envMap[$key] = $value
>> "%ENV_RENDER_PS%" echo }
>> "%ENV_RENDER_PS%" echo $output = foreach ^($key in $orderedKeys^) { '{0}={1}' -f $key, $envMap[$key] }
>> "%ENV_RENDER_PS%" echo Set-Content -LiteralPath $dst -Value $output -Encoding ASCII

powershell -NoProfile -ExecutionPolicy Bypass -File "%ENV_RENDER_PS%"
set "PS_STATUS=%errorlevel%"
del /q "%ENV_RENDER_PS%" >nul 2>nul
if not "%PS_STATUS%"=="0" (
    echo Failed to prepare merged environment file.
    exit /b 1
)

echo Prepared environment file: %MERGED_ENV_FILE%
exit /b 0

:end_success
if defined DEPLOY_REMOTE (
    if exist "%MERGED_ENV_FILE%" del /q "%MERGED_ENV_FILE%" >nul 2>nul
)
echo.
echo ==========================================
echo   Build Complete!
echo ==========================================
call :maybe_pause
exit /b 0

:maybe_pause
if defined NONINTERACTIVE exit /b 0
pause
exit /b 0
