# Clash Mini 一键静默发行脚本 (Windows PowerShell)
# 用法: .\scripts\release.ps1 <版本号>
# 示例: .\scripts\release.ps1 1.4.5
#
# 效果: 用户仅需点击一次 Submit 确认，脚本全自动完成：
#   1. 版本号更新 (Python)
#   2. git commit + tag + push (触发 CI)
#   3. 每 3 分钟轮询 CI 状态（自动纠错重试）
#   4. CI 成功后下载 setup.exe 到 portable_test/ 并验收
#
# 约束:
#   - Token 从 github_token.txt 读取并设为环境变量，绝不打印
#   - 不接触代理进程 (verge-mihomo/clash-verge) 及端口 (10801/9098)
#   - 无任何 Read-Host 交互，版本号必须由参数传入

param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# ─────────────────────────────────────────────
# 配置常量
# ─────────────────────────────────────────────
$ProjectRoot   = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$GitHubRepo    = "qiu-yuxiao/clash-mini"
$TokenFile     = Join-Path $ProjectRoot "github_token.txt"
$PortableDir   = Join-Path $ProjectRoot "portable_test"
$PollInterval  = 180   # 秒（3 分钟）
$MaxRetries    = 2     # CI 失败自动重试次数（网络类）
$QueueTimeout  = 900   # 排队超时（15 分钟）

# ─────────────────────────────────────────────
# 输出函数
# ─────────────────────────────────────────────
function Log-Step  { param($msg) Write-Host "`n>>> $msg" -ForegroundColor Cyan }
function Log-Ok    { param($msg) Write-Host "  [OK]   $msg" -ForegroundColor Green }
function Log-Warn  { param($msg) Write-Host "  [WARN] $msg" -ForegroundColor Yellow }
function Log-Error { param($msg) Write-Host "  [ERR]  $msg" -ForegroundColor Red }
function Log-Info  { param($msg) Write-Host "  [..]   $msg" }

# 时间戳前缀（用于进度行）
function Now { return (Get-Date -Format "HH:mm") }

# ─────────────────────────────────────────────
# 阶段 0：Token 读取（最高优先级，绝不暴露）
# ─────────────────────────────────────────────
Log-Step "读取 GitHub Token"
if (-not (Test-Path $TokenFile)) {
    Log-Error "github_token.txt 不存在: $TokenFile"
    exit 1
}
# 仅设为环境变量，后续 gh 命令自动使用，不打印不传参
$env:GH_TOKEN = (Get-Content $TokenFile -Raw -Encoding UTF8).Trim()
Log-Ok "Token 已加载至环境变量 GH_TOKEN（值已隐藏）"

# ─────────────────────────────────────────────
# 阶段 1：前置检查
# ─────────────────────────────────────────────
Log-Step "前置检查"

# 版本号格式
$Version = $Version.TrimStart('v')
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Log-Error "版本号格式错误: $Version（应为 x.y.z，例如 1.4.5）"
    exit 1
}
$TagName = "v$Version"
Log-Ok "目标版本: $Version  |  Tag: $TagName"

# 当前分支必须是 dev
$Branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($Branch -ne "dev") {
    Log-Error "当前分支为 '$Branch'，发行必须在 dev 分支执行"
    exit 1
}
Log-Ok "分支检查通过: dev"

# 工作区必须干净
$Dirty = git status --porcelain
if ($Dirty) {
    Log-Error "工作区存在未提交的改动，请先提交后再发行:"
    git status --short
    exit 1
}
Log-Ok "工作区干净"

# 确认本地无未推送提交
$Unpushed = git log origin/dev..dev --oneline
if ($Unpushed) {
    Log-Error "存在未推送到远端的本地提交，请先 git push origin dev:"
    Write-Host $Unpushed
    exit 1
}
Log-Ok "本地提交已全部推送"

# Tag 冲突检查
$ExistingTag = git tag -l $TagName
if ($ExistingTag) {
    Log-Error "Tag $TagName 已存在，若需重新发行请先手动删除:"
    Log-Error "  git tag -d $TagName"
    Log-Error "  git push origin :refs/tags/$TagName"
    exit 1
}
Log-Ok "Tag $TagName 不存在，可继续"

# ─────────────────────────────────────────────
# 阶段 2：版本号更新（Python）
# ─────────────────────────────────────────────
Log-Step "更新版本号文件（Python）"
$BumpScript = Join-Path $ProjectRoot "scripts\bump_version.py"
python $BumpScript $Version
if ($LASTEXITCODE -ne 0) {
    Log-Error "bump_version.py 执行失败"
    exit 1
}

# ─────────────────────────────────────────────
# 阶段 3：Git 提交 + 推送 + 打 Tag
# ─────────────────────────────────────────────
Log-Step "Git 提交并推送"

git config --local http.sslBackend openssl
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "release: bump version to $Version" --no-verify
if ($LASTEXITCODE -ne 0) { Log-Error "git commit 失败"; exit 1 }

git push origin dev --no-verify
if ($LASTEXITCODE -ne 0) { Log-Error "git push dev 失败"; exit 1 }
Log-Ok "dev 分支已推送"

git tag $TagName
git push origin $TagName --no-verify
if ($LASTEXITCODE -ne 0) { Log-Error "git push tag 失败"; exit 1 }
Log-Ok "Tag $TagName 已推送 -> CI 已触发"

git config --local --unset http.sslBackend

Write-Host ""
Write-Host "  CI 监控面板: https://github.com/$GitHubRepo/actions" -ForegroundColor Cyan

# ─────────────────────────────────────────────
# 阶段 4：轮询 CI 状态（每 3 分钟）
# ─────────────────────────────────────────────
Log-Step "等待 CI 构建完成（每 3 分钟汇报一次）"

function Get-RunInfo {
    # 获取最新 workflow run 的 id、status、conclusion
    $json = gh api "repos/$GitHubRepo/actions/workflows/release.yml/runs?per_page=5" 2>$null
    if (-not $json) { return $null }
    $obj = $json | ConvertFrom-Json
    $run = $obj.workflow_runs | Where-Object { $_.head_branch -eq "dev" -or $_.head_sha -ne $null } |
           Sort-Object -Property id -Descending | Select-Object -First 1
    return $run
}

function Get-RunLogs {
    param($RunId)
    $url = "https://github.com/$GitHubRepo/actions/runs/$RunId"
    return $url
}

$AttemptCount = 0
$MaxAttempts  = $MaxRetries + 1
$StartTime    = Get-Date
$LastRunId    = $null

:outerLoop while ($AttemptCount -lt $MaxAttempts) {
    $AttemptCount++
    if ($AttemptCount -gt 1) {
        Log-Warn "第 $AttemptCount 次尝试（自动重试）..."
        # 重新推送 tag 触发新 CI run
        git tag -d $TagName 2>$null
        git push origin ":refs/tags/$TagName" --no-verify 2>$null
        Start-Sleep -Seconds 5
        git tag $TagName
        git push origin $TagName --no-verify
        if ($LASTEXITCODE -ne 0) { Log-Error "重推 Tag 失败，放弃重试"; break }
        Log-Ok "Tag 已重推，等待新 CI Run 启动..."
        Start-Sleep -Seconds 30
        $StartTime = Get-Date
    }

    # 等待 CI run 出现
    $WaitSec = 0
    $Run     = $null
    while (-not $Run -or $Run.id -eq $LastRunId) {
        Start-Sleep -Seconds 15
        $WaitSec += 15
        $Run = Get-RunInfo
        if ($WaitSec -ge 120) {
            Log-Warn "等待 CI Run 出现超时（2 分钟），继续等待..."
        }
    }
    $LastRunId = $Run.id
    Log-Info "CI Run ID: $($Run.id) | $(Get-RunLogs $Run.id)"

    # 轮询 run 状态
    :pollLoop while ($true) {
        $Elapsed = [int]((Get-Date) - $StartTime).TotalMinutes
        $Run     = Get-RunInfo

        if (-not $Run) {
            Log-Warn "$(Now) API 返回空，稍后重试..."
            Start-Sleep -Seconds $PollInterval
            continue
        }

        $Status     = $Run.status      # queued / in_progress / completed
        $Conclusion = $Run.conclusion  # success / failure / cancelled / null

        Write-Host "  [$(Now)] CI: $Status$(if ($Conclusion) { " / $Conclusion" }) | 已运行: $Elapsed 分钟"

        if ($Status -eq "completed") {
            if ($Conclusion -eq "success") {
                Log-Ok "CI 构建成功！总耗时: $Elapsed 分钟"
                break outerLoop
            }
            elseif ($Conclusion -eq "failure" -or $Conclusion -eq "cancelled") {
                $LogUrl = Get-RunLogs $Run.id
                # 判断是否可重试（网络类失败，通过失败的 job 名称粗判）
                $FailedJobs = gh api "repos/$GitHubRepo/actions/runs/$($Run.id)/jobs" 2>$null |
                              ConvertFrom-Json | Select-Object -ExpandProperty jobs |
                              Where-Object { $_.conclusion -eq "failure" }
                $IsRetryable = $false
                foreach ($job in $FailedJobs) {
                    $jname = $job.name.ToLower()
                    if ($jname -match "build" -and $AttemptCount -lt $MaxAttempts) {
                        # build job 失败且仍有重试次数 -> 视为可能是瞬时错误，重试
                        $IsRetryable = $true
                    }
                }

                if ($IsRetryable) {
                    Log-Warn "CI 失败（结论: $Conclusion），准备自动重试..."
                    Log-Warn "失败详情: $LogUrl"
                    Start-Sleep -Seconds 10
                    break pollLoop  # 跳回 outerLoop 重试
                }
                else {
                    Log-Error "CI 构建失败（结论: $Conclusion），无法自动恢复"
                    Log-Error "请手动查看失败日志: $LogUrl"
                    Log-Error "常见原因: 编译错误、签名失败、代码问题，需人工介入"
                    exit 1
                }
            }
            else {
                Log-Error "CI 结束但结论未知: $Conclusion"
                exit 1
            }
        }

        # 排队超时检查
        if ($Status -eq "queued" -and $Elapsed -ge ($QueueTimeout / 60)) {
            Log-Warn "CI 排队超时（$Elapsed 分钟），自动重推 Tag..."
            break pollLoop
        }

        Start-Sleep -Seconds $PollInterval
    }
}

if ($AttemptCount -ge $MaxAttempts) {
    Log-Error "已达最大重试次数（$MaxRetries 次），发行失败，请人工介入"
    exit 1
}

# ─────────────────────────────────────────────
# 阶段 5：验收 - 等待 Release 正式发布
# ─────────────────────────────────────────────
Log-Step "等待 Release 从 Draft 转为正式发布"

$ReleaseReady = $false
for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Seconds 15
    $ReleaseJson = gh api "repos/$GitHubRepo/releases/tags/$TagName" 2>$null
    if ($ReleaseJson) {
        $Rel = $ReleaseJson | ConvertFrom-Json
        if (-not $Rel.draft) {
            $ReleaseReady = $true
            Log-Ok "Release $TagName 已正式发布"
            break
        }
        else {
            Log-Info "Release 仍为 Draft，继续等待..."
        }
    }
    else {
        Log-Info "Release 尚未创建，继续等待..."
    }
}

if (-not $ReleaseReady) {
    Log-Warn "等待 Release 发布超时，尝试直接下载..."
}

# ─────────────────────────────────────────────
# 阶段 6：下载 setup.exe 到 portable_test/ 并验收
# ─────────────────────────────────────────────
Log-Step "下载发行产物并验收"

if (-not (Test-Path $PortableDir)) {
    New-Item -ItemType Directory -Path $PortableDir | Out-Null
}

$SetupPattern = "*_x64-setup.exe"
gh release download $TagName `
    --pattern $SetupPattern `
    --dir $PortableDir `
    --clobber `
    --repo $GitHubRepo

if ($LASTEXITCODE -ne 0) {
    Log-Error "setup.exe 下载失败，请手动检查 Release 页面"
    exit 1
}

# 查找下载的文件
$SetupFile = Get-ChildItem $PortableDir -Filter "*${Version}*x64-setup.exe" |
             Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $SetupFile) {
    Log-Error "未在 $PortableDir 中找到 setup.exe"
    exit 1
}

$FileSizeMB = [math]::Round($SetupFile.Length / 1MB, 2)
if ($SetupFile.Length -eq 0) {
    Log-Error "下载的 setup.exe 文件大小为 0，产物异常"
    exit 1
}

# ─────────────────────────────────────────────
# 完成报告
# ─────────────────────────────────────────────
$TotalMin = [int]((Get-Date) - $StartTime).TotalMinutes
Write-Host ""
Write-Host ("=" * 62) -ForegroundColor Green
Write-Host "  发行成功！" -ForegroundColor Green
Write-Host ("=" * 62) -ForegroundColor Green
Write-Host ""
Write-Host "  版本:    $TagName"
Write-Host "  文件:    $($SetupFile.Name)  ($FileSizeMB MB)"
Write-Host "  路径:    $($SetupFile.FullName)"
Write-Host "  Release: https://github.com/$GitHubRepo/releases/tag/$TagName"
Write-Host "  总耗时:  $TotalMin 分钟"
Write-Host ""
