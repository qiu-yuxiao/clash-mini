# Stage 2 Compliance and Dead Code Audit Report

**Project**: Clash Mini (under `c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge`)  
**Date**: July 10, 2026  
**Auditor / Agent**: `teamwork_preview_worker`  

---

## Executive Summary

This report presents the findings of the **Stage 2 Compliance and Dead Code Audit** conducted on the Clash Mini codebase. The objective of this audit is to identify dead code, interface mismatches between the Tauri backend and React frontend, hardcoded timeout values violating the project's latency constraints (2000ms/3000ms limits), and architectural gaps relative to planned robustness improvements. 

---

## 1. Tauri Backend-Frontend Command Mismatch and Dead Commands

### 1.1 Critical Name Mismatch Bug (`get_app_uptime`)
A severe runtime IPC defect exists in the application status display. The React frontend attempts to invoke a command named `get_app_uptime` to show the application's uptime, but the Tauri backend registers it under a different name.

* **Frontend Call ([c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\services\cmds.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/cmds.ts#L423))**:
  ```typescript
  export const getAppUptime = async () => {
    return invoke<number>('get_app_uptime')
  }
  ```
* **Rust Backend Registration ([c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L137))**:
  ```rust
  crate::utils::sysinfo::get_app_uptime_cmd,
  ```
* **Rust Backend Definition ([c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\utils\sysinfo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/sysinfo.rs#L163))**:
  ```rust
  #[tauri::command]
  pub fn get_app_uptime_cmd(state: State<'_, RwLock<Platform>>) -> Result<u128, String>
  ```
* **Impact**: When the frontend attempts to display uptime status, the call to `get_app_uptime` immediately throws a runtime IPC error because the backend only handles `get_app_uptime_cmd`. This breaks the status UI component.

---

### 1.2 Unused and Dead Tauri Commands
A total of **50 Tauri commands** are registered on the backend inside `generate_handlers()` in [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L134) but are never imported or invoked anywhere in the frontend codebase (`src/`). These commands represent legacy features from Clash Verge Rev that were stripped from Clash Mini (e.g., local/WebDAV backups, DNS configuration, and profile ordering).

Below is the complete registry of these 50 unused commands:

| # | Command Name | Rust Implementation File | File Line / Source | Original Purpose / Legacy Feature |
|---|---|---|---|---|
| 1 | `get_system_info` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\utils\sysinfo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/sysinfo.rs#L158) | Line 158 | Retrieves full system metadata. |
| 2 | `export_diagnostic_info` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\utils\sysinfo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/sysinfo.rs#L173) | Line 173 | Copies system diagnostic string to clipboard. |
| 3 | `open_app_dir` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L9) | Line 9 | Opens the application home directory in explorer. |
| 4 | `open_web_url` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L31) | Line 31 | Opens a web URL in the system browser. |
| 5 | `open_app_log` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L43) | Line 43 | Opens the app log file (Tray menu item). |
| 6 | `open_core_log` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L53) | Line 53 | Opens the Clash core log file (Tray menu item). |
| 7 | `get_portable_flag` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L87) | Line 87 | Checks if portable mode is enabled. |
| 8 | `get_network_interfaces` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L79) | Line 79 (re-export) | Lists network interfaces. |
| 9 | `get_system_hostname` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L64) | Line 64 (re-export) | Retrieves system hostname. |
| 10 | `restart_app` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L80) | Line 80 | Restarts the main application shell. |
| 11 | `start_core` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L33) | Line 33 | Starts the Clash core process. |
| 12 | `stop_core` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L39) | Line 39 | Stops the Clash core process. |
| 13 | `get_auto_launch_status` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L100) | Line 100 | Checks if auto-launch is configured. |
| 14 | `entry_lightweight_mode` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/lightweight.rs#L5) | Line 5 | Enters minimalist UI mode. |
| 15 | `exit_lightweight_mode` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\lightweight.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/lightweight.rs#L11) | Line 11 | Exits minimalist UI mode. |
| 16 | `reinstall_service` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/service.rs#L19) | Line 19 (re-export) | Reinstalls the helper system service. |
| 17 | `repair_service` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\service.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/service.rs#L25) | Line 25 (re-export) | Repairs the helper system service. |
| 18 | `change_clash_core` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L141) | Line 141 | Switches the active Clash core executable. |
| 19 | `get_runtime_yaml` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L207) | Line 207 | Fetches raw runtime YAML config. |
| 20 | `get_runtime_exists` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L199) | Line 199 | Checks if runtime config file exists. |
| 21 | `get_runtime_logs` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L223) | Line 223 | Fetches runtime logs from Clash core. |
| 22 | `get_runtime_proxy_chain_config` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L238) | Line 238 | Fetches proxy chain runtime config. |
| 23 | `invoke_uwp_tool` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\uwp.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/uwp.rs#L5) | Line 5 | Invokes the UWP loopback tool. |
| 24 | `copy_clash_env` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L80) | Line 80 | Prepares environment config for Clash. |
| 25 | `save_dns_config` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\network.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/network.rs#L126) | Line 126 | Saves DNS settings to YAML. |
| 26 | `apply_dns_config` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\network.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/network.rs#L181) | Line 181 (re-export) | Applies custom DNS configuration. |
| 27 | `check_dns_config_exists` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\network.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/network.rs#L182) | Line 182 (re-export) | Verifies if DNS config file exists. |
| 28 | `get_dns_config_content` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\network.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/network.rs#L183) | Line 183 (re-export) | Reads DNS config file content. |
| 29 | `validate_dns_config` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\network.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/network.rs#L184) | Line 184 (re-export) | Validates DNS syntax and rules. |
| 30 | `test_delay` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L113) | Line 113 | Backend custom URL latency test. |
| 31 | `get_app_dir` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L93) | Line 93 | Retrieves App Home directory path. |
| 32 | `copy_icon_file` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L112) | Line 112 | Copies icon asset to runtime cache. |
| 33 | `exit_app` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L74) | Line 74 | Performs application shutdown. |
| 34 | `get_network_interfaces_info` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L85) | Line 85 | Lists detailed network interface information. |
| 35 | `create_profile` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L119) | Line 119 | Generates a new connection profile. |
| 36 | `reorder_profile` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L103) | Line 103 | Modifies profile order settings. |
| 37 | `get_next_update_time` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L386) | Line 386 | Gets schedule info for profile updates. |
| 38 | `script_validate_notice` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L18) | Line 18 (in validate) | Notifies script validation state. |
| 39 | `validate_script_file` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L25) | Line 25 (in validate) | Validates merge script JavaScript syntax. |
| 40 | `create_local_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\backup.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/backup.rs#L7) | Line 7 | Creates a compressed local backup zip. |
| 41 | `list_local_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\backup.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/backup.rs#L13) | Line 13 | Lists available local backup archives. |
| 42 | `delete_local_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\backup.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/backup.rs#L19) | Line 19 | Deletes a local backup archive. |
| 43 | `restore_local_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\backup.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/backup.rs#L25) | Line 25 | Restores application state from a zip. |
| 44 | `import_local_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\backup.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/backup.rs#L31) | Line 31 | Imports a backup file. |
| 45 | `export_local_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\backup.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/backup.rs#L37) | Line 37 | Exports a backup file to external storage. |
| 46 | `create_webdav_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\webdav.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/webdav.rs#L6) | Line 6 (re-export) | Uploads backup file to a WebDAV host. |
| 47 | `save_webdav_config` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\webdav.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/webdav.rs#L7) | Line 7 (re-export) | Saves WebDAV sync credentials. |
| 48 | `list_webdav_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\webdav.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/webdav.rs#L8) | Line 8 (re-export) | Lists files stored on the WebDAV host. |
| 49 | `delete_webdav_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\webdav.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/webdav.rs#L9) | Line 9 (re-export) | Deletes a backup from WebDAV storage. |
| 50 | `restore_webdav_backup` | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\webdav.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/webdav.rs#L10) | Line 10 (re-export) | Downloads and restores from WebDAV. |

---

## 2. Residual Legacy Speedtest/Latency Remnants

In Clash Mini, the speedtest/latency logic is restricted: the legacy timeout field (`default_latency_timeout`) is deprecated in favor of a unified max delay threshold (`NODE_DELAY_MAX_MS = 2000`). However, vestigial code paths and structures still remain.

### 2.1 Configuration Field Remnant (`default_latency_timeout`)
* **Backend Definition ([c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\config\verge.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/verge.rs#L149))**:
  - **Line 149**: `pub default_latency_timeout: Option<i16>,`
  - **Line 526**: `patch!(default_latency_timeout);`
  - **Status**: The backend config struct still defines, parses, and serializes this field despite it not being used anywhere in the Rust core proxy engine or speedtest logic.
* **Frontend Hook Mismatch ([c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\hooks\use-proxy-delay-state.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-proxy-delay-state.ts#L87))**:
  - **Code**:
    ```typescript
    const currentTimeout = getPreloadConfig()?.default_latency_timeout || NODE_DELAY_MAX_MS
    const result = await delayManager.checkDelay(proxy.name, groupName, currentTimeout)
    ```
  - **Status**: The React hook still fetches this configuration field and falls back to `NODE_DELAY_MAX_MS`. To comply with constraints, this logic should ignore `default_latency_timeout` entirely and consistently pass `NODE_DELAY_MAX_MS`.

---

### 2.2 Obsolete `DelayManager.isBatchTesting` Dead Branches
The `isBatchTesting` getter under `DelayManager` has been hardcoded to return `false` as batch testing states are handled differently in Clash Mini:
* **Definition ([c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\services\delay.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/services/delay.ts#L41))**:
  ```typescript
  get isBatchTesting(): boolean {
    return false
  }
  ```
Because this getter always returns `false`, there are multiple dead code branches in the React component tree checking this property:

1. **[c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\components\layout\resize-handles.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/components/layout/resize-handles.tsx#L108)**:
   ```typescript
   if (DelayManager.isBatchTesting) return
   ```
2. **[c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L264)**:
   ```typescript
   if (!DelayManager.isBatchTesting) {
   ```
3. **[c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L299)**:
   ```typescript
   if (!DelayManager.isBatchTesting) {
   ```
4. **[c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\_layout\components\active-node-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx#L133)**:
   ```typescript
   if (delayManager.isBatchTesting) return
   ```
5. **[c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\_layout\components\active-node-card.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout/components/active-node-card.tsx#L153)**:
   ```typescript
   if (delayManager.isBatchTesting) return
   ```
6. **[c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\providers\window\window-provider.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/providers/window/window-provider.tsx#L175)**:
   ```typescript
   !DelayManager.isBatchTesting
   ```

---

## 3. Non-Aligned Timeout and Delay Thresholds

To align with Clash Mini specifications, delay and timeout bounds should conform to either a **2000ms** (strict delay threshold) or **3000ms** (connection/validation timeout grace) limit. Six specific values violate this threshold:

| # | File Path & Line | Target Logic | Current Value | Target Limit | Rationale / Recommended Fix |
|---|---|---|---|---|---|
| 1 | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\feat\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L121) & [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\feat\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L163) | Custom URL Latency Test Timeout | `10000ms` (10s) | `2000ms` or `3000ms` | Reduces wait time on broken proxies during manual URL latency tests. Align to `2000ms` / `3000ms`. |
| 2 | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\core\validate.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs#L374) | Clash Core Configuration Validation | `5000ms` (5s) | `3000ms` | Prevents the UI from lagging when trying to load/validate corrupt configurations. Reduce to `3000ms`. |
| 3 | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\enhance\script.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/script.rs#L16) | User JS Merge Script Timeout | `5000ms` (5s) | `3000ms` | Prevents malicious/infinite loops in custom user profiles scripts from blocking core start. Reduce to `3000ms`. |
| 4 | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\unlock.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/unlock.tsx#L226) | Media Unlock Check Timeout | `15000ms` (15s) | `3000ms` | Media check should fail fast to prevent lengthy loading spinners on blocked connections. Reduce to `3000ms`. |
| 5 | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\hooks\use-system-state.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-system-state.ts#L23) | Startup Service Grace Period | `10000ms` (10s) | `3000ms` | Long grace period delays fallback to system proxy when UAC fails or helper service is unresponsive. Reduce to `3000ms`. |
| 6 | [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L313) | Fallback Batch Speed Test Delay | `6000ms` (6s) | `2000ms` or `3000ms` | The layout fallback trigger should respond faster when nodes fail to return metrics in batch testing. Reduce to `2000ms` / `3000ms`. |

---

## 4. Architecture Complexity & Code Smells

### 4.1 Unimplemented [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\STARTUP_ALIGNMENT_PLAN.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/STARTUP_ALIGNMENT_PLAN.md#L1)
A dedicated blueprint file ([c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\STARTUP_ALIGNMENT_PLAN.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/STARTUP_ALIGNMENT_PLAN.md#L1)) exists at the root of the repository, laying out a 6-step plan to integrate defensive locks, exit guards, and idempotency checks into Clash Mini's core engine. 
However, **none of these steps are implemented**:
* `CoreManager` in [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\core\manager\mod.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/mod.rs#L1) lacks the `lifecycle_lock` mutex and the `config_update_in_progress` atomic flag.
* `start_core`, `stop_core`, and `restart_core` in [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\core\manager\lifecycle.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/manager/lifecycle.rs#L1) execute raw asynchronous procedures without acquiring locks.
* **Consequences**: This omissions allow race conditions. Rapid toggles of system proxy/TUN mode or quick successive restarts can lead to overlapping core execution processes, duplicate `mini-mihomo` instances, port collision errors, and UI freezes.

### 4.2 Obsolete TODO Comments in [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\cmd\app.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/app.rs#L40)
* **Lines 40 & 50**:
  ```rust
  // TODO 后续可以为前端提供接口，当前作为托盘菜单使用
  ```
* **Context**: These comments suggest that the frontend could be provided with direct API endpoints for opening app/core logs. In Clash Mini, the tray and log interfaces are deliberately simplified, making these TODOs obsolete legacy leftovers from Clash Verge Rev.

---

## 5. Conclusions & Next Steps

1. **Uptime Command**: Fix the name mismatch by renaming the backend Tauri command `get_app_uptime_cmd` to `get_app_uptime` (in [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\utils\sysinfo.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/sysinfo.rs#L163) and [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L137)) or aligning the frontend invocation parameter.
2. **Dead Command Cleanup**: Safely deregister the 50 unused commands in [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\lib.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L137) and purge their corresponding module definitions to minimize binary size.
3. **Legacy speedtest/latency Purge**: Remove `default_latency_timeout` from [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\config\verge.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/verge.rs#L149) and enforce `NODE_DELAY_MAX_MS` in the React frontend. Clean up dead code branches dependent on `DelayManager.isBatchTesting`.
4. **Timeout Alignment**: Reduce all six hardcoded timeout/delay bounds (in [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\feat\clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L121), [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src-tauri\src\enhance\script.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/enhance/script.rs#L16), [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\unlock.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/unlock.tsx#L226), [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\hooks\use-system-state.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/hooks/use-system-state.ts#L23), and [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\src\pages\_layout.tsx](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src/pages/_layout.tsx#L313)) to `2000ms` or `3000ms` targets.
5. **Startup Plan Execution**: Implement Steps 1 and 2 of the [c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge\STARTUP_ALIGNMENT_PLAN.md](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/STARTUP_ALIGNMENT_PLAN.md#L1) to introduce the Mutex and Atomic guards protecting core lifecycle actions.
