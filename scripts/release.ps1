# Clash Mini 规范发布脚本 (Windows PowerShell)
# 用法:
#   .\scripts\release.ps1              # 交互式：询问版本号
#   .\scripts\release.ps1 1.3.9       # 直接指定版本号
#   .\scripts\release.ps1 1.3.9 -NoPush  # 本地准备，不推送（用于测试）

param(
    [string]$Version,
    [switch]$NoPush,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

# ============ 配置 ============
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$GitHubRepo = "qiu-yuxiao/clash-mini"
$PrivateKeyPath = Join-Path $ProjectRoot "tauri-key"
$PrivateKeyPassword = "clash-mini-auto-update-2026"

# ============ 颜色输出函数 ============
function Write-Step($msg) { Write-Host "`n▶ $msg" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "  ✅ $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "  ⚠️  $msg" -ForegroundColor Yellow }
function Write-Err($msg)  { Write-Host "  ❌ $msg" -ForegroundColor Red }

# ============ 前置检查 ============
Write-Step "前置检查"

# 检查私钥文件
if (-not (Test-Path $PrivateKeyPath)) {
    Write-Err "私钥文件不存在: $PrivateKeyPath"
    Write-Err "请先运行: pnpm tauri signer generate --out-file tauri-key"
    exit 1
}
Write-Ok "私钥文件存在: $PrivateKeyPath"

# 检查 git 状态
Set-Location $ProjectRoot
$gitStatus = git status --porcelain
if ($gitStatus) {
    Write-Warn "工作区有未提交的改动:"
    git status --short
    $confirm = Read-Host "是否继续？(y/N)"
    if ($confirm -ne "y" -and $confirm -ne "Y") {
        Write-Err "已取消"
        exit 1
    }
}

# 检查是否在 dev 分支
$gitBranch = git rev-parse --abbrev-ref HEAD
if ($gitBranch -ne "dev") {
    Write-Warn "当前分支: $gitBranch (不是 dev)"
    $confirm = Read-Host "是否在 dev 分支上发布？(y/N)"
    if ($confirm -ne "y" -and $confirm -ne "Y") {
        Write-Err "已取消"
        exit 1
    }
}

# ============ 版本号 ============
Write-Step "版本号"

if (-not $Version) {
    $pkg = Get-Content (Join-Path $ProjectRoot "package.json") | ConvertFrom-Json
    $currentVersion = $pkg.version
    Write-Host "  当前版本: $currentVersion"
    $Version = Read-Host "  请输入新版本号 (例如 1.3.9)"
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Err "版本号格式错误，应为 x.y.z（例如 1.3.9）"
    exit 1
}

$tagName = "v$Version"
Write-Ok "新版本: $Version (tag: $tagName)"

# 检查 tag 是否已存在
$existingTag = git tag -l $tagName
if ($existingTag) {
    Write-Err "Tag $tagName 已存在"
    $confirm = Read-Host "是否删除旧 tag 并重新发布？(y/N)"
    if ($confirm -ne "y" -and $confirm -ne "Y") {
        Write-Err "已取消"
        exit 1
    }
    git tag -d $tagName
    git push origin ":$tagName" 2>$null
    Write-Ok "已删除远程旧 tag"
}

# ============ 更新版本号文件 ============
Write-Step "更新版本号文件"

function Update-VersionInFile($filePath, $pattern, $replacement) {
    $content = Get-Content $filePath -Raw
    if ($content -match $pattern) {
        $newContent = $content -replace $pattern, $replacement
        Set-Content -Path $filePath -Value $newContent -NoNewline
        Write-Ok "已更新: $filePath"
    } else {
        Write-Warn "未找到匹配项: $filePath (pattern: $pattern)"
    }
}

# package.json
$pkgPath = Join-Path $ProjectRoot "package.json"
$pkg = Get-Content $pkgPath | ConvertFrom-Json
$pkg.version = $Version
$pkg | ConvertTo-Json -Compress | Set-Content -Path $pkgPath -NoNewline
Write-Ok "已更新: package.json"

# tauri.conf.json
$tauriPath = Join-Path $ProjectRoot "src-tauri\tauri.conf.json"
$tauri = Get-Content $tauriPath -Raw
$tauri = $tauri -replace '"version":\s*"\d+\.\d+\.\d+"', "`"version`": `"$Version`""
Set-Content -Path $tauriPath -Value $tauri -NoNewline
Write-Ok "已更新: tauri.conf.json"

# Cargo.toml
$cargoPath = Join-Path $ProjectRoot "src-tauri\Cargo.toml"
Update-VersionInFile $cargoPath 'version = "\d+\.\d+\.\d+"' "version = `"$Version`""

# Cargo.lock (自动更新，不需要手动改)
Write-Ok "版本号文件已全部更新"

# ============ 构建 ============
if (-not $SkipBuild) {
    Write-Step "构建 Tauri 应用 (setup 版)"

    # 安装依赖
    Write-Host "  安装依赖..."
    pnpm i
    if ($LASTEXITCODE -ne 0) { Write-Err "pnpm i 失败"; exit 1 }

    # 预构建
    Write-Host "  预构建..."
    pnpm run prebuild x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { Write-Err "prebuild 失败"; exit 1 }

    # Web 构建
    Write-Host "  Web 构建..."
    pnpm run web:build
    if ($LASTEXITCODE -ne 0) { Write-Err "web:build 失败"; exit 1 }

    # Tauri 构建（生成 setup.exe）
    Write-Host "  Tauri 构建（生成 NSIS 安装包）..."
    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $PrivateKeyPath -Raw
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $PrivateKeyPassword
    pnpm tauri build --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { Write-Err "tauri build 失败"; exit 1 }

    Write-Ok "构建完成"
} else {
    Write-Warn "跳过构建 (-SkipBuild)"
}

# ============ 签名 setup.exe ============
Write-Step "签名 setup.exe"

$setupExe = Join-Path $ProjectRoot "src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\Clash.Mini_${Version}_x64-setup.exe"
if (-not (Test-Path $setupExe)) {
    # 尝试找文件（版本号可能在文件名里不同）
    $setupExe = Get-ChildItem (Join-Path $ProjectRoot "src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\") -Filter "*.exe" | Select-Object -First 1
    if ($setupExe) {
        $setupExe = $setupExe.FullName
    }
}

if (-not (Test-Path $setupExe)) {
    Write-Err "未找到 setup.exe，请检查构建输出"
    Write-Host "  期望路径: $setupExe"
    exit 1
}
Write-Ok "找到 setup.exe: $setupExe"

# 签名
Write-Host "  签名中..."
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $PrivateKeyPath -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $PrivateKeyPassword
pnpm tauri signer sign $setupExe
if ($LASTEXITCODE -ne 0) { Write-Err "签名失败"; exit 1 }

$sigFile = "$setupExe.sig"
if (-not (Test-Path $sigFile)) {
    Write-Err "签名文件未生成: $sigFile"
    exit 1
}
Write-Ok "签名成功: $sigFile"

# ============ 更新 app-update.json ============
Write-Step "更新 app-update.json"

$updateJsonPath = Join-Path $ProjectRoot "updater\app-update.json"

# 读取签名
$sigContent = Get-Content $sigFile -Raw
$sigB64 = $sigContent.Trim() -replace "`n","" -replace "`r",""

# 获取文件大小
$fileSize = (Get-Item $setupExe).Length

# 生成发布日期
$pubDate = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ").Replace("T", "T")

# 读取 bug_list.md 获取更新说明（如果有）
$notes = "Clash Mini v$Version"
$bugListPath = Join-Path $ProjectRoot "bug_list.md"
if (Test-Path $bugListPath) {
    $bugContent = Get-Content $bugListPath -Raw
    if ($bugContent -match "## 📌 待验证与活动中 Bug 详情.*?### (BUG-\d+):\s*(.*?)(?=###|##)") {
        $notes = "Clash Mini v$Version — $($Matches[1]): $($Matches[2].Trim())"
    }
}

# 写入 app-update.json
$updateJson = @{
    version    = $Version
    notes      = $notes
    pub_date   = $pubDate
    platforms  = @{
        "windows-x86_64" = @{
            signature = $sigB64
            url       = "https://github.com/$GitHubRepo/releases/download/$tagName/Clash.Mini_${Version}_x64-setup.exe"
            size      = $fileSize
        }
    }
}

$updateJson | ConvertTo-Json -Compress | Set-Content -Path $updateJsonPath -NoNewline -Encoding UTF8
Write-Ok "app-update.json 已更新"
Write-Host "  URL:  https://github.com/$GitHubRepo/releases/download/$tagName/Clash.Mini_${Version}_x64-setup.exe"
Write-Host "  Size: $fileSize bytes"
Write-Host "  Sig:  $($sigB64.Substring(0, 40))..."

# ============ 提交并推送 ============
Write-Step "提交代码"

# 添加所有版本变更文件
git add package.json
git add src-tauri/tauri.conf.json
git add src-tauri/Cargo.toml
git add src-tauri/Cargo.lock
git add updater/app-update.json

# 检查是否有变更
$diffStatus = git diff --cached --shortstat
if (-not $diffStatus) {
    Write-Warn "没有需要提交的变更"
} else {
    $commitMsg = "release: bump version to $Version"
    git commit -m $commitMsg --no-verify
    Write-Ok "已提交: $commitMsg"
}

# 创建 tag
git tag $tagName
Write-Ok "已创建 tag: $tagName"

if ($NoPush) {
    Write-Warn "跳过推送 (-NoPush)"
    Write-Host "`n下次推送时执行:"
    Write-Host "  git push origin dev"
    Write-Host "  git push origin $tagName`n"
} else {
    Write-Step "推送到 GitHub"
    git push origin dev
    if ($LASTEXITCODE -ne 0) { Write-Err "推送 dev 分支失败"; exit 1 }

    git push origin $tagName
    if ($LASTEXITCODE -ne 0) { Write-Err "推送 tag 失败"; exit 1 }

    Write-Ok "已推送到 GitHub"
    Write-Host "`nCI 将自动构建，请访问以下链接查看进度:"
    Write-Host "  https://github.com/$GitHubRepo/actions`n"
}

# ============ 完成 ============
Write-Host "`n" + ("=" * 60) -ForegroundColor Green
Write-Host "  🎉 发布准备完成！" -ForegroundColor Green
Write-Host ("=" * 60) -ForegroundColor Green
Write-Host "`n版本:    $Version"
Write-Host "Tag:     $tagName"
Write-Host "Setup:   Clash.Mini_${Version}_x64-setup.exe"
Write-Host "大小:    $([math]::Round($fileSize / 1MB, 2)) MB`n"

if (-not $NoPush) {
    Write-Host "下一步:"
    Write-Host "  1. 等待 CI 构建完成 (约 20-30 分钟)"
    Write-Host "  2. CI 完成后，setup.exe 和 .sig 会自动上传到 Release"
    Write-Host "  3. 用户打开 v$(([version]$Version).Build - 1) 版本时会收到自动更新提示`n"
}

Write-Host "下载链接 (构建完成后生效):"
Write-Host "  https://github.com/$GitHubRepo/releases/tag/$tagName`n"
