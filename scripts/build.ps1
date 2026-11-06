# Roblox Account Manager — сборка под Windows (NSIS-установщик + portable zip)
# Использование: powershell -ExecutionPolicy Bypass -File scripts\build.ps1

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "==> Проверка зависимостей" -ForegroundColor Cyan
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js не найден. Установите Node.js 18+ с https://nodejs.org"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust не найден. Установите через rustup: https://rustup.rs"
}

Write-Host "==> npm install" -ForegroundColor Cyan
npm install
if ($LASTEXITCODE -ne 0) { throw "npm install завершился с ошибкой" }

Write-Host "==> Сборка приложения (фронтенд + Rust + NSIS)" -ForegroundColor Cyan
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "Сборка завершилась с ошибкой" }

$releaseDir = Join-Path $root "release"
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

# 1) Установщик NSIS
Get-ChildItem "src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue |
    Copy-Item -Destination $releaseDir -Force

# 2) Portable-архив с исполняемым файлом
$exe = Get-ChildItem "src-tauri\target\release\*.exe" |
    Where-Object { $_.Name -notmatch "-uninstall|setup" } |
    Select-Object -First 1
if ($exe) {
    $portable = Join-Path $env:TEMP "RAM-portable"
    Remove-Item $portable -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $portable | Out-Null
    Copy-Item $exe.FullName (Join-Path $portable $exe.Name)
    Compress-Archive -Path (Join-Path $portable "*") `
        -DestinationPath (Join-Path $releaseDir "RobloxAccountManager-portable.zip") -Force
}

Write-Host ""
Write-Host "Готово! Артефакты в папке release\:" -ForegroundColor Green
Get-ChildItem $releaseDir | Format-Table Name, Length -AutoSize
