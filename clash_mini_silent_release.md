# 👑 Clash Mini One-Click Silent Release Guidelines

> [!IMPORTANT]
> **These guidelines are for AI Agent automatic releases. Any unexpected dialogs or workflow errors caused by violating these rules will be considered a critical failure.**
>
> **[Kernel Release Guideline] All release versions decouple the mandatory binding of the old kernel, and default to dynamically pulling and using the official latest released stable kernel (or Prerelease-Alpha) for packaging and release.**

---

## 🔐 Token Usage Rule (Highest Priority)

- GitHub API token **must be read from the `github_token.txt` file in the project root directory**.
- It must be used by **setting environment variables** (`$env:GH_TOKEN = ...`). It is strictly forbidden to print or output the token value to logs or embed it in command-line strings.
- Token reading example (the only compliant way):
  ```powershell
  $env:GH_TOKEN = (Get-Content 'github_token.txt' -Raw).Trim()
  # After this, all gh commands automatically use this environment variable without passing it as an argument.
  ```

---

## 🛡️ Proxy Server Protection Rule (Highest Priority)

- The local execution environment runs a TUN mode proxy server, and **the release workflow requires no proxy configuration**.
- **It is strictly forbidden** to kill or interfere with the following processes and ports: `verge-mihomo`, `clash-verge`, `mini-mihomo`, ports `10801`, `9098`.
- Release scripts must not contain any `Stop-Process` or `taskkill` commands targeting the above processes.

---

## 🚦 Standard Release Path (The Only Recommended Flow)

### Trigger Condition
Local code has been fully committed, and a new version needs to be released.

### One-Click Execution
```powershell
.\scripts\release.ps1 <version>
# Example: .\scripts\release.ps1 1.4.5
```

The user **only needs to click Submit once** to confirm, and the script will automatically complete all of the following phases without manual intervention:

---

## 🃏 Phase 1: Local Preparation (~1 minute)

The script executes automatically without any interactive prompts:

1. **Token Load**: Read from `github_token.txt` and set to `$env:GH_TOKEN` (silent, does not expose value).
2. **Pre-flight Checks**:
   - Confirm currently on `dev` branch.
   - Confirm local workspace is clean (no uncommitted changes).
   - Confirm version number format is valid (`x.y.z`).
   - Confirm tag `v<version>` does not conflict with existing tags.
3. **Version Update**: Call `python scripts/bump_version.py <version>` to update the following three files:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
4. **Git Operations (Executed Silently)**:
   ```
   git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
   git commit -m "release: bump version to <version>" --no-verify
   git push origin dev --no-verify
   git tag v<version>
   git push origin v<version> --no-verify
   ```
5. **Print Actions Monitoring Link**: `https://github.com/qiu-yuxiao/clash-mini/actions`

> [!WARNING]
> **Pushing any new commits to the `dev` branch is prohibited from the time `git push origin v<version>` triggers the CI until the CI `update_dev` job finishes.**
> Reason: The CI `update_dev` job automatically generates and force-pushes `app-update.json` to `dev`. Any prior `git push origin dev` might conflict with or overwrite the CI commit.

---

## 🃠 Phase 2: Cloud Build Monitoring (Polled every 3 minutes, ~30 minutes)

The script queries the Actions run status via the GitHub API, printing progress **every 3 minutes**:

```
[12:05] ⏳ CI Status: in_progress | Elapsed: 3 min | Stage: build
[12:08] ⏳ CI Status: in_progress | Elapsed: 6 min | Stage: build
...
[12:32] ✅ CI Status: completed | Conclusion: success | Total Time: 27 min
```

### Automatic Error Resolution

| Failure Type | Auto-handling Method |
|---|---|
| Network timeout / temporary API error | Wait 3 minutes and retry polling, up to 2 retries |
| CI Run queue timeout (> 15 minutes before starting) | Delete tag and push again to trigger a new Run, up to 1 retry |
| Build failure (compile/signature error) | Print the full error log URL, stop and report, requires manual intervention |

---

## 🏁 Phase 3: Verification & Completion (~2 minutes)

After the CI succeeds, the script automatically:

1. Waits for the Draft Release to be published (polls for `draft=false`).
2. Downloads `setup.exe` to `portable_test/` directory.
3. Verifies that the file size is > 0 (confirms the artifact is valid).
4. Prints final report:
   ```
   ✅ Release Succeeded!
   Version: v1.4.5
   File: portable_test/Clash.Mini_1.4.5_x64-setup.exe (42.3 MB)
   Release: https://github.com/qiu-yuxiao/clash-mini/releases/tag/v1.4.5
   ```

---

## 🔧 Emergency Rollback Plan (Only used when release.ps1 is completely unavailable)

> [!CAUTION]
> The following is a manual emergency operation and **should absolutely not be used under normal circumstances**. Every step must be executed manually, and the operator assumes all risks.

1. **Update Version**: Manually modify version fields in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`.
2. **Register Docs**: Record new features in `clash_mini_agreements.md` and update bug status in `bug_list.md`.
3. **Git Commit**:
   ```powershell
   git add .
   git commit -m "release: bump version to <version>" --no-verify
   git push origin dev --no-verify
   git tag v<version>
   git push origin v<version> --no-verify
   ```
4. **Monitor CI**: Manually visit `https://github.com/qiu-yuxiao/clash-mini/actions` to inspect build status.
5. **Download Artifact**:
   ```powershell
   $env:GH_TOKEN = (Get-Content 'github_token.txt' -Raw).Trim()
   gh release download v<version> --pattern '*_x64-setup.exe' --dir 'portable_test' --clobber --repo qiu-yuxiao/clash-mini
   ```
6. **Update Git Config** (If push fails, configure OpenSSL and retry):
   ```powershell
   git config --local http.sslBackend openssl
   # Restore after push:
   git config --local --unset http.sslBackend
   ```

---

## ⚙️ GitHub Release Workflow Specs (Workflow Architecture)

`release.yml` is the **only official release pipeline** for Clash Mini, triggered only when pushing a version tag `git push v<version>`.

### Job Dependency Chain (Four-Step Deadlock-Free Design)

```
validate → build → update_dev → publish
```

- `validate`: Quickly verifies tag source (restricted to dev branch) and version consistency (~2 minutes).
- `build`: Compiles Tauri on Windows x64 in the cloud, packages setup.exe + portable.zip, uploads them as a Draft Release, and signs and uploads the `.sig` automatically (~25-30 minutes).
- `update_dev`: Downloads the assets from the release, automatically generates `app-update.json` (with correct signature/size/url), and commits and force-pushes it to dev (~1 minute).
- `publish`: Extracts release notes from Changelog.md and publishes the Draft Release to official (~1 minute).

> [!IMPORTANT]
> **The maintenance of `app-update.json` belongs entirely to the CI's `update_dev` job. `release.ps1` does not generate or commit this file.**
> The CI will automatically write the correct signature, file size, and download URL after the build is successful.

### Release Assets (Since v1.3.8)

**Official release assets are limited to the following two items for Windows x64:**

- **[Required]** `Clash.Mini_<version>_x64-setup.exe` (NSIS installer, supports Tauri auto-update).
- **[Optional]** `Clash.Mini_<version>_x64_portable.zip` (Portable green version, `continue-on-error: true`).

**The following assets must not be released under any circumstances:**
- ❌ Linux builds (.deb / .rpm / .AppImage)
- ❌ macOS builds (.dmg)
- ❌ ARM Windows builds
- ❌ WebView2 fixed version

### Prohibitions

- Using official version numbers in the `TAG_NAME` of `autobuild.yml` is strictly forbidden; autobuild must use a fixed `autobuild` tag.
- Mixing conditionally skipped jobs as `needs` dependencies in `release.yml` is strictly forbidden.
- Including non-Windows platform artifact links in official Release release notes is strictly forbidden.

---

## 📦 Release Artifact Description (Since v1.3.8)

### `Clash.Mini_<version>_x64-setup.exe` (Primary Asset)

- NSIS installer, supports Tauri auto-updater.
- Users run the installer, and the program is installed in `C:\Users\<username>\AppData\Local\Programs\clash-mini\`.
- Auto-updates download the new `setup.exe` and run the installer automatically.

### `Clash.Mini_<version>_x64_portable.zip` (Optional Asset)

- Portable green version, ready to run after extraction.
- **Does not support auto-updates** (Tauri updater does not support ZIP on Windows).
- Provided only for users who do not want to install and prefer manual downloads.

---

## 🔐 Signature and Auto-Update Specs

### Private Key Management

- Private key file: `tauri-key` (stored in the project root, added to `.gitignore`, strictly forbidden from being committed to Git).
- Public key file: `tauri-key.pub` (content written in `tauri.conf.json`'s `updater.pubkey`).
- The CI automatically reads the two GitHub Secrets `TAURI_PRIVATE_KEY` and `TAURI_KEY_PASSWORD` to complete the signature during build.

### app-update.json Automated Maintenance

- The CI `update_dev` job automatically generates and commits this file after each successful build, requiring no manual actions.
- Ensure the `signature` field is identical to the content of the `.sig` file (guaranteed by CI).
- Ensure the `url` field points to the correct Release download link (guaranteed by CI).
- Ensure the `size` field matches the actual file size in bytes (guaranteed by CI).

---

## 📝 Release Checklist

Confirm before release:

- [ ] Local code is fully committed and pushed (`git log origin/dev..dev` has no output).
- [ ] `clash_mini_agreements.md` has registered the new features/modifications (and has been committed separately).
- [ ] `bug_list.md` has updated the Bug statuses (and has been committed separately).
- [ ] `Changelog.md` has added the release notes for this version (used by CI to auto-generate Release Notes).
- [ ] Version numbers are consistent in `package.json`, `tauri.conf.json`, and `Cargo.toml` (automatically completed by `release.ps1`).
- [ ] CI build succeeded and the Release is published (not Draft).
- [ ] The `setup.exe` of this version exists in `portable_test/` and the file size is normal (automatically validated by `release.ps1`).
- [ ] `https://raw.githubusercontent.com/qiu-yuxiao/clash-mini/dev/updater/app-update.json` is accessible on GitHub.
- [ ] Older clients can receive auto-update notifications.
