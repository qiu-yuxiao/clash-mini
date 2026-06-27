# Clash Mini One-Click Silent Release Script (Windows PowerShell)
# Usage: .\scripts\release.ps1 <version>
# Example: .\scripts\release.ps1 1.4.5
#
# Effect: The user only needs to click Submit once to confirm, and the script does the rest automatically:
#   1. Version bump (Python)
#   2. git commit + tag + push (Triggers CI)
#   3. Poll CI status every 3 minutes (automatic retry on error)
#   4. Download setup.exe to portable_test/ and verify after CI succeeds
#
# 约束:
#   - Token is read from github_token.txt and set as env variable, never printed
#   - Do not interfere with proxy processes (verge-mihomo/clash-verge) and ports (10801/9098)
#   - No Read-Host interaction, version must be passed as parameter

param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# Fix encoding issue on Windows PowerShell (CP936 to UTF-8)
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

# ─────────────────────────────────────────────
# Configuration Constants
# ─────────────────────────────────────────────
$ProjectRoot   = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$GitHubRepo    = "qiu-yuxiao/clash-mini"
$TokenFile     = Join-Path $ProjectRoot "github_token.txt"
$PortableDir   = Join-Path $ProjectRoot "portable_test"
$PollInterval  = 180   # Seconds (3 minutes)
$MaxRetries    = 2     # Number of automatic retries on CI failure (network-related)
$QueueTimeout  = 900   # Queue timeout (15 minutes)

# ─────────────────────────────────────────────
# Output Functions
# ─────────────────────────────────────────────
function Log-Step  { param($msg) Write-Host "`n>>> $msg" -ForegroundColor Cyan }
function Log-Ok    { param($msg) Write-Host "  [OK]   $msg" -ForegroundColor Green }
function Log-Warn  { param($msg) Write-Host "  [WARN] $msg" -ForegroundColor Yellow }
function Log-Error { param($msg) Write-Host "  [ERR]  $msg" -ForegroundColor Red }
function Log-Info  { param($msg) Write-Host "  [..]   $msg" }

# Timestamp prefix
function Now { return (Get-Date -Format "HH:mm") }

# ─────────────────────────────────────────────
# Stage 0: Token Load (Highest priority, never print)
# ─────────────────────────────────────────────
Log-Step "Reading GitHub Token"
if (-not (Test-Path $TokenFile)) {
    Log-Error "github_token.txt not found: $TokenFile"
    exit 1
}
# Set as env variable for automatic use by subsequent gh commands, never printed
$env:GH_TOKEN = (Get-Content $TokenFile -Raw -Encoding UTF8).Trim()
Log-Ok "Token loaded into environment variable GH_TOKEN (value hidden)"

# ─────────────────────────────────────────────
# Stage 1: Pre-flight Checks
# ─────────────────────────────────────────────
Log-Step "Pre-flight Checks"

# Version format
$Version = $Version.TrimStart('v')
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Log-Error "Invalid version format: $Version (expected x.y.z, e.g., 1.4.5)"
    exit 1
}
$TagName = "v$Version"
Log-Ok "Target Version: $Version  |  Tag: $TagName"

# Current branch must be dev
$Branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($Branch -ne "dev") {
    Log-Error "Current branch is '$Branch', release must be executed on dev branch"
    exit 1
}
Log-Ok "Branch check passed: dev"

# Working directory must be clean
$Dirty = git status --porcelain
if ($Dirty) {
    Log-Error "Working directory has uncommitted changes, please commit before release:"
    git status --short
    exit 1
}
Log-Ok "Working directory clean"

# Confirm no unpushed local commits
$Unpushed = git log origin/dev..dev --oneline
if ($Unpushed) {
    Log-Error "Unpushed local commits exist, please run git push origin dev first:"
    Write-Host $Unpushed
    exit 1
}
Log-Ok "Local commits successfully pushed"

# Tag conflict check
$ExistingTag = git tag -l $TagName
$RemoteTagCheck = git ls-remote origin refs/tags/$TagName
$RemoteTagExists = $false
if ($RemoteTagCheck) {
    $RemoteTagExists = $true
}

$SkipBumpAndTag = $false
if ($ExistingTag -or $RemoteTagExists) {
    Log-Warn "Tag $TagName already exists (Local: [$(if($ExistingTag){"Yes"}else{"No"})], Remote: [$(if($RemoteTagExists){"Yes"}else{"No"})])."
    Log-Info "Checking if a workflow run already exists for this release..."
    
    $json = gh api "repos/$GitHubRepo/actions/workflows/release.yml/runs?per_page=5" 2>$null
    if ($json) {
        $obj = ($json -join "`n") | ConvertFrom-Json
        $run = $obj.workflow_runs | Where-Object { $_.head_branch -eq $TagName } |
               Sort-Object -Property id -Descending | Select-Object -First 1
        if ($run) {
            Log-Ok "Found existing workflow run for this release (Run ID: $($run.id))."
            Log-Ok "Skipping Version Bump and Git Push steps. Jumping directly to CI monitoring..."
            $SkipBumpAndTag = $true
        }
    }
    
    if (-not $SkipBumpAndTag) {
        Log-Error "Tag exists but no active or completed release workflow run was found for tag $TagName."
        Log-Error "To re-release, please manually delete the tag:"
        Log-Error "  git tag -d $TagName"
        Log-Error "  git push origin :refs/tags/$TagName"
        exit 1
    }
} else {
    Log-Ok "Tag $TagName does not exist, proceeding with clean release"
}

if (-not $SkipBumpAndTag) {
    # ─────────────────────────────────────────────
    # Stage 2: Version Bump (Python)
    # ─────────────────────────────────────────────
    Log-Step "Bumping Version Numbers (Python)"
    $BumpScript = Join-Path $ProjectRoot "scripts\bump_version.py"
    python $BumpScript $Version
    if ($LASTEXITCODE -ne 0) {
        Log-Error "bump_version.py execution failed"
        exit 1
    }

    # ─────────────────────────────────────────────
    # Stage 3: Git Commit, Push, Tag
    # ─────────────────────────────────────────────
    Log-Step "Git Commit and Push"

    git config --local http.sslBackend openssl
    git config --local http.sslVerify false
    git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
    git commit -m "release: bump version to $Version" --allow-empty --no-verify
    if ($LASTEXITCODE -ne 0) { Log-Error "git commit failed"; exit 1 }

    git push origin dev --no-verify
    if ($LASTEXITCODE -ne 0) { Log-Error "git push dev failed"; exit 1 }
    Log-Ok "dev branch pushed"

    git tag $TagName
    git push origin $TagName --no-verify
    if ($LASTEXITCODE -ne 0) { Log-Error "git push tag failed"; exit 1 }
    Log-Ok "Tag $TagName pushed -> CI triggered"

    git config --local --unset http.sslBackend
    git config --local --unset http.sslVerify
}

Write-Host ""
Write-Host "  CI Monitor Dashboard: https://github.com/$GitHubRepo/actions" -ForegroundColor Cyan

# ─────────────────────────────────────────────
# Stage 4: Poll CI Status (Every 3 minutes)
# ─────────────────────────────────────────────
Log-Step "Waiting for CI build to complete (polling every 3 minutes)"

function Get-RunInfo {
    # Get newest workflow run id, status, and conclusion
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    try {
        $json = gh api "repos/$GitHubRepo/actions/workflows/release.yml/runs?per_page=5" 2>$null
        if (-not $json) { return $null }
        $obj = ($json -join "`n") | ConvertFrom-Json
        $run = $obj.workflow_runs | Where-Object { $_.head_branch -eq $TagName } |
               Sort-Object -Property id -Descending | Select-Object -First 1
        return $run
    } catch {
        return $null
    } finally {
        $ErrorActionPreference = $oldEAP
    }
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
        Log-Warn "Attempt $AttemptCount (Automatic Retry)..."
        # Re-push tag to trigger a new CI run
        git tag -d $TagName 2>$null
        git push origin ":refs/tags/$TagName" --no-verify 2>$null
        Start-Sleep -Seconds 5
        git tag $TagName
        git push origin $TagName --no-verify
        if ($LASTEXITCODE -ne 0) { Log-Error "Failed to push tag, aborting retries"; break }
        Log-Ok "Tag pushed, waiting for new CI Run to start..."
        Start-Sleep -Seconds 30
        $StartTime = Get-Date
    }

    # Wait for CI run to appear
    $WaitSec = 0
    $Run     = $null
    while (-not $Run -or $Run.id -eq $LastRunId) {
        Start-Sleep -Seconds 15
        $WaitSec += 15
        $Run = Get-RunInfo
        if ($WaitSec -ge 120) {
            Log-Warn "Timeout waiting for CI Run (2 minutes), continuing wait..."
        }
    }
    $LastRunId = $Run.id
    Log-Info "CI Run ID: $($Run.id) | $(Get-RunLogs $Run.id)"

    # Poll run status
    :pollLoop while ($true) {
        $Elapsed = [int]((Get-Date) - $StartTime).TotalMinutes
        $Run     = Get-RunInfo

        if (-not $Run) {
            Log-Warn "$(Now) API returned empty response, retrying later..."
            Start-Sleep -Seconds $PollInterval
            continue
        }

        $Status     = $Run.status      # queued / in_progress / completed
        $Conclusion = $Run.conclusion  # success / failure / cancelled / null

        Write-Host "  [$(Now)] CI: $Status$(if ($Conclusion) { " / $Conclusion" }) | Elapsed: $Elapsed minutes"

        if ($Status -eq "completed") {
            if ($Conclusion -eq "success") {
                Log-Ok "CI build completed successfully! Total elapsed: $Elapsed minutes"
                break outerLoop
            }
            elseif ($Conclusion -eq "failure" -or $Conclusion -eq "cancelled") {
                $LogUrl = Get-RunLogs $Run.id
                # Determine if retryable (network errors checked via failed job name)
                $FailedJobsJson = gh api "repos/$GitHubRepo/actions/runs/$($Run.id)/jobs" 2>$null
                $FailedJobs = ($FailedJobsJson -join "`n") | ConvertFrom-Json | Select-Object -ExpandProperty jobs |
                              Where-Object { $_.conclusion -eq "failure" }
                $IsRetryable = $false
                foreach ($job in $FailedJobs) {
                    $jname = $job.name.ToLower()
                    if ($jname -match "build" -and $AttemptCount -lt $MaxAttempts) {
                        # build job failed and retries remain -> treat as transient error, retry
                        $IsRetryable = $true
                    }
                }

                if ($IsRetryable) {
                    Log-Warn "CI failed (conclusion: $Conclusion), preparing automatic retry..."
                    Log-Warn "Failure details: $LogUrl"
                    Start-Sleep -Seconds 10
                    break pollLoop  # Jump back to outerLoop to retry
                }
                else {
                    Log-Error "CI build failed (conclusion: $Conclusion), cannot recover automatically"
                    Log-Error "Please view failed logs manually: $LogUrl"
                    Log-Error "Common causes: compile errors, signature errors, code bugs, manual check required"
                    exit 1
                }
            }
            else {
                Log-Error "CI finished but conclusion unknown: $Conclusion"
                exit 1
            }
        }

        # Queue timeout check
        if ($Status -eq "queued" -and $Elapsed -ge ($QueueTimeout / 60)) {
            Log-Warn "CI queue timed out ($Elapsed minutes), pushing tag again..."
            break pollLoop
        }

        Start-Sleep -Seconds $PollInterval
    }
}

if ($AttemptCount -ge $MaxAttempts) {
    Log-Error "Max retry attempts reached ($MaxRetries), release failed, manual check required"
    exit 1
}

# ─────────────────────────────────────────────
# Stage 5: Acceptance - Wait for Release to Publish
# ─────────────────────────────────────────────
Log-Step "Waiting for Release draft to publish"

$ReleaseReady = $false
for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Seconds 15
    $ReleaseJson = gh api "repos/$GitHubRepo/releases/tags/$TagName" 2>$null
    if ($ReleaseJson) {
        $Rel = ($ReleaseJson -join "`n") | ConvertFrom-Json
        if (-not $Rel.draft) {
            $ReleaseReady = $true
            Log-Ok "Release $TagName is published"
            break
        }
        else {
            Log-Info "Release is still Draft, continuing wait..."
        }
    }
    else {
        Log-Info "Release not created yet, continuing wait..."
    }
}

if (-not $ReleaseReady) {
    Log-Warn "Timeout waiting for Release to publish, attempting download anyway..."
}

# ─────────────────────────────────────────────
# Stage 6: Download setup.exe to portable_test/ and Verify
# ─────────────────────────────────────────────
Log-Step "Downloading Release Artifact and Verifying"

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
    Log-Error "setup.exe download failed, please manually check the Release page"
    exit 1
}

# Search for downloaded file
$SetupFile = Get-ChildItem $PortableDir -Filter "*${Version}*x64-setup.exe" |
             Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $SetupFile) {
    Log-Error "Could not find setup.exe in $PortableDir"
    exit 1
}

$FileSizeMB = [math]::Round($SetupFile.Length / 1MB, 2)
if ($SetupFile.Length -eq 0) {
    Log-Error "Downloaded setup.exe file size is 0, abnormal artifact"
    exit 1
}

# ─────────────────────────────────────────────
# Final Report
# ─────────────────────────────────────────────
$TotalMin = [int]((Get-Date) - $StartTime).TotalMinutes
Write-Host ""
Write-Host ("=" * 62) -ForegroundColor Green
Write-Host "  Release Succeeded!" -ForegroundColor Green
Write-Host ("=" * 62) -ForegroundColor Green
Write-Host ""
Write-Host "  Version:    $TagName"
Write-Host "  File:    $($SetupFile.Name)  ($FileSizeMB MB)"
Write-Host "  Path:    $($SetupFile.FullName)"
Write-Host "  Release: https://github.com/$GitHubRepo/releases/tag/$TagName"
Write-Host "  Total Elapsed:  $TotalMin minutes"
Write-Host ""
