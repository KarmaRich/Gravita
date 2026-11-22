# Gravita (引力下载器) Windows Release 一键编译与打包脚本
# 用法: powershell -ExecutionPolicy Bypass -File .\scripts\package.ps1

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

$Version = "0.1.0"
$ReleaseName = "Gravita-v$Version-win64"
$DistDir = Join-Path $RootDir "dist"
$OutDir = Join-Path $DistDir $ReleaseName
$ZipPath = Join-Path $DistDir "$ReleaseName.zip"

Write-Host "==> [1/4] 正在以 Release 优化配置编译 Gravita.exe (LTO + Strip + GUI Subsystem)..." -ForegroundColor Cyan
cargo build -p idm-desktop --bin Gravita --release
if ($LASTEXITCODE -ne 0) {
    throw "Cargo Release 编译失败！"
}

Write-Host "==> [2/4] 正在清理并创建发布目录: $OutDir ..." -ForegroundColor Cyan
if (Test-Path $OutDir) {
    Remove-Item -Recurse -Force $OutDir
}
if (Test-Path $ZipPath) {
    Remove-Item -Force $ZipPath
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "resources\bin") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "extension\src") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "extension\icons") | Out-Null

Write-Host "==> [3/4] 正在复制桌面主程序、内置 aria2c 引擎与浏览器扩展..." -ForegroundColor Cyan
Copy-Item (Join-Path $RootDir "target\release\Gravita.exe") -Destination (Join-Path $OutDir "Gravita.exe")
Copy-Item (Join-Path $RootDir "resources\bin\aria2c.exe") -Destination (Join-Path $OutDir "resources\bin\aria2c.exe")
Copy-Item (Join-Path $RootDir "resources\gravita.ico") -Destination (Join-Path $OutDir "resources\gravita.ico")
Copy-Item (Join-Path $RootDir "README.md") -Destination (Join-Path $OutDir "README.md")

# 复制浏览器扩展生产文件（剔除单元测试 filter.test.js 与 package.json）
Copy-Item (Join-Path $RootDir "extension\manifest.json") -Destination (Join-Path $OutDir "extension\manifest.json")
Copy-Item (Join-Path $RootDir "extension\popup.html") -Destination (Join-Path $OutDir "extension\popup.html")
Copy-Item (Join-Path $RootDir "extension\icons\*.png") -Destination (Join-Path $OutDir "extension\icons\")
Get-ChildItem (Join-Path $RootDir "extension\src\*.js") | Where-Object { $_.Name -notlike "*.test.js" } | ForEach-Object {
    Copy-Item $_.FullName -Destination (Join-Path $OutDir "extension\src\$($_.Name)")
}

# 额外打包一份单独的浏览器扩展 zip 方便分发
$ExtZipPath = Join-Path $OutDir "Gravita-Browser-Extension.zip"
Compress-Archive -Path (Join-Path $OutDir "extension\*") -DestinationPath $ExtZipPath -Force

Write-Host "==> [4/4] 正在生成便携版发行压缩包: $ZipPath ..." -ForegroundColor Cyan
Compress-Archive -Path "$OutDir\*" -DestinationPath $ZipPath -Force

$ExeSizeMb = [Math]::Round((Get-Item (Join-Path $OutDir "Gravita.exe")).Length / 1MB, 2)
$ZipSizeMb = [Math]::Round((Get-Item $ZipPath).Length / 1MB, 2)

Write-Host ""
Write-Host "✔ 打包完成！" -ForegroundColor Green
Write-Host "  - 主程序路径: $OutDir\Gravita.exe ($ExeSizeMb MB)"
Write-Host "  - 浏览器扩展: $OutDir\extension (及 $ExtZipPath)"
Write-Host "  - 完整便携包: $ZipPath ($ZipSizeMb MB)"
