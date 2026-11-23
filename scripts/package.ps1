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
$ExtStageDir = Join-Path $DistDir "extension-stage"
$ExtZipPath = Join-Path $DistDir "Gravita-Browser-Extension.zip"
$ExtCrxPath = Join-Path $DistDir "Gravita-Browser-Extension.crx"

Write-Host "==> [1/4] 正在以 Release 优化配置编译 Gravita.exe (LTO + Strip + GUI Subsystem)..." -ForegroundColor Cyan
cargo build -p idm-desktop --bin Gravita --release
if ($LASTEXITCODE -ne 0) {
    throw "Cargo Release 编译失败！"
}

Write-Host "==> [2/4] 正在清理并创建发布目录: $OutDir ..." -ForegroundColor Cyan
foreach ($p in @($OutDir, $ZipPath, $ExtStageDir, $ExtZipPath, $ExtCrxPath)) {
    if (Test-Path $p) {
        Remove-Item -Recurse -Force $p
    }
}
New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "resources\bin") | Out-Null

Write-Host "==> [3/4] 正在组装精简客户端目录 (仅含 Gravita.exe 与 resources\bin)..." -ForegroundColor Cyan
Copy-Item (Join-Path $RootDir "target\release\Gravita.exe") -Destination (Join-Path $OutDir "Gravita.exe")
Copy-Item (Join-Path $RootDir "resources\bin\aria2c.exe") -Destination (Join-Path $OutDir "resources\bin\aria2c.exe")
if (Test-Path (Join-Path $RootDir "resources\bin\ca-certificates.crt")) {
    Copy-Item (Join-Path $RootDir "resources\bin\ca-certificates.crt") -Destination (Join-Path $OutDir "resources\bin\ca-certificates.crt")
}

# 独立打包浏览器扩展 (.zip 与 .crx 输出在 dist/ 根目录，不混入客户端安装目录)
New-Item -ItemType Directory -Force -Path (Join-Path $ExtStageDir "src") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $ExtStageDir "icons") | Out-Null
Copy-Item (Join-Path $RootDir "extension\manifest.json") -Destination (Join-Path $ExtStageDir "manifest.json")
Copy-Item (Join-Path $RootDir "extension\popup.html") -Destination (Join-Path $ExtStageDir "popup.html")
Copy-Item (Join-Path $RootDir "extension\icons\*.png") -Destination (Join-Path $ExtStageDir "icons\")
Get-ChildItem (Join-Path $RootDir "extension\src\*.js") | Where-Object { $_.Name -notlike "*.test.js" } | ForEach-Object {
    Copy-Item $_.FullName -Destination (Join-Path $ExtStageDir "src\$($_.Name)")
}
Compress-Archive -Path (Join-Path $ExtStageDir "*") -DestinationPath $ExtZipPath -Force
Remove-Item -Recurse -Force $ExtStageDir

node (Join-Path $RootDir "scripts\pack-crx.mjs") $ExtZipPath $ExtCrxPath
if ($LASTEXITCODE -ne 0) {
    throw "生成浏览器扩展 .crx 包失败！"
}

Write-Host "==> [4/4] 正在生成便携版发行压缩包: $ZipPath ..." -ForegroundColor Cyan
Compress-Archive -Path "$OutDir\*" -DestinationPath $ZipPath -Force

$ExeSizeMb = [Math]::Round((Get-Item (Join-Path $OutDir "Gravita.exe")).Length / 1MB, 2)
$ZipSizeMb = [Math]::Round((Get-Item $ZipPath).Length / 1MB, 2)

Write-Host ""
Write-Host "✔ 打包完成！" -ForegroundColor Green
Write-Host "  - 客户端目录: $OutDir (仅含 Gravita.exe [$ExeSizeMb MB] 与 resources\bin)"
Write-Host "  - 客户端便携包: $ZipPath ($ZipSizeMb MB)"
Write-Host "  - 独立插件包: $ExtZipPath 及 $ExtCrxPath"
