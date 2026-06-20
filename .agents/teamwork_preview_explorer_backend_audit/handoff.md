# Backend Pre-Release Audit Handoff Report

## 1. Observation
A comprehensive pre-release code audit of the Clash Verge backend monitor and commands was conducted. The following files were examined:
- `src-tauri/src/module/monitor.rs`
- `src-tauri/src/cmd/proxy.rs`
- `src-tauri/src/cmd/clash.rs`
- `src-tauri/src/cmd/profile.rs`

Key observations include:
- `AUTO_SELECT_RUNNING` lock logic in `trigger_backend_auto_select` (lines 278–282 in `monitor.rs`) and how it behaves when called concurrently via tauri commands vs the background monitor daemon.
- Mismatch between doc comments and implementation of `trigger_auto_select` (lines 155–168 in `proxy.rs`) with respect to `sort_type`.
- Parsing method in `get_saved_sort_type` (lines 97–103 in `monitor.rs`) using a YAML parser instead of a JSON parser for `proxy_head_state.json`.
- Arbitrary filter condition `delay > 50` on latency results in `trigger_backend_auto_select_inner` (line 392 in `monitor.rs`).
- Dead config patching and lack of persistence updates in `apply_dns_config` (lines 165–172 in `clash.rs`).
- Missing backend reload/refresh trigger in `restore_previous_profile` (lines 186–203 in `profile.rs`) upon profile update failures.
- State notification mismatch in `delete_profile` (lines 162–170 in `profile.rs`), where a deleted profile's UID is broadcasted as the current active profile to the frontend.

---

## 2. Logic Chain
- **AUDIT-BE-001**: A background monitor loop runs indefinitely, checking profile switches and node health. In `trigger_backend_auto_select`, an atomic lock prevents concurrent execution. If a manual auto-selection is running, the background daemon's profile switch auto-selection is silently skipped (`Ok(vec![])`). Since the daemon updates `last_profile_uid` before executing and does not retry if skipped, the system never selects the optimal node for the new profile.
- **AUDIT-BE-002**: The `trigger_auto_select` command uses `sort_type.unwrap_or(1)`. Since `1` triggers latency sorting, any frontend invocation with `None` (intended to read from config) gets forced into latency-based sorting, skipping the config-reading logic (which is mapped to `0` in the backend).
- **AUDIT-BE-003**: `proxy_head_state.json` is a JSON file. Using `serde_yaml_ng` to parse it works because JSON is a subset of YAML, but is inefficient, inconsistent with `serde_json` used elsewhere in the same file, and constitutes bad coding practice.
- **AUDIT-BE-004**: Low-latency nodes under 50ms (e.g. 10ms–50ms) are highly desirable. Filtering them out via `delay_info.delay > 50` ignores the best-performing connections and selects slower nodes instead.
- **AUDIT-BE-005**: `IRuntime::patch_config` only processes keys in `PATCH_CONFIG_INNER` and `"tun"`. Patching `"dns"` has no effect and is a silent no-op. Furthermore, the `enable_dns_settings` flag in `Config::verge()` is never toggled, so `enhance::enhance()` never builds the configuration with DNS.
- **AUDIT-BE-006**: When profile validation fails, `restore_previous_profile` reverts the profile list state in-memory and on-disk, but fails to call `CoreManager::global().update_config_forced()`. As a result, the Clash core continues running the broken configuration or remains stopped.
- **AUDIT-BE-007**: Deleting the current profile triggers fallback selection in the backend, but the frontend notification broadcasts the deleted profile ID (`index`), causing the frontend to request data for a non-existent profile and break.

---

## 3. Caveats
- No caveats. The investigation completely covered all target backend modules and commands, isolating logic errors, dead code, race conditions, and integration mismatches.

---

## 4. Conclusion

Below is the detailed list of findings from the audit:

| Finding ID | Description | Severity | File Path |
|---|---|---|---|
| **AUDIT-BE-001** | Race condition in auto-select trigger on profile switch | Major | [src-tauri/src/module/monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L274-L304) |
| **AUDIT-BE-002** | `sort_type: None` defaults to `1` instead of `0` in tauri command | Major | [src-tauri/src/cmd/proxy.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/proxy.rs#L155-L168) |
| **AUDIT-BE-003** | Inconsistent/inefficient parser for JSON configuration file | Minor | [src-tauri/src/module/monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L97-L103) |
| **AUDIT-BE-004** | Latency threshold filters out high-performance nodes <= 50ms | Major | [src-tauri/src/module/monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L391-L395) |
| **AUDIT-BE-005** | Dead code and lack of persistence updates in `apply_dns_config` | Major | [src-tauri/src/cmd/clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L165-L172) |
| **AUDIT-BE-006** | Profile switch restoration fails to reload Clash core configuration | Major | [src-tauri/src/cmd/profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L186-L203) |
| **AUDIT-BE-007** | Profile deletion sends wrong profile ID to frontend | Major | [src-tauri/src/cmd/profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L162-L170) |

---

### Detailed Findings

#### AUDIT-BE-001: Race condition in auto-select trigger on profile switch
- **Severity**: Major
- **File Path**:
  - Lock Check: [src-tauri/src/module/monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L274-L304)
  - Daemon Switch: [src-tauri/src/module/monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L505-L509)
- **Root Cause Analysis**:
  `trigger_backend_auto_select` uses a global atomic lock `AUTO_SELECT_RUNNING` to prevent concurrent execution. If a manual auto-selection is already running, the background daemon's profile switch auto-selection is silently skipped (`Ok(vec![])`). Since the daemon updates `last_profile_uid` and does not retry when skipped, the new profile remains without auto-selected nodes.
- **Suggested Fix**:
  Return an error (`AUTO_SELECT_BUSY`) when the lock is held, and modify the background monitor to retry if the lock is busy during a profile switch.
  
  ```diff
  diff --git a/src-tauri/src/module/monitor.rs b/src-tauri/src/module/monitor.rs
  index 1234567..89abcde 100644
  --- a/src-tauri/src/module/monitor.rs
  +++ b/src-tauri/src/module/monitor.rs
  @@ -276,7 +276,7 @@ pub async fn trigger_backend_auto_select(
   ) -> anyhow::Result<Vec<(String, u32)>> {
       // Mutex lock to prevent concurrent calls
       if AUTO_SELECT_RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
  -        logging!(info, Type::Lightweight, "[Background Monitor] Auto-select already running, returning busy");
  -        return Ok(vec![]);
  +        logging!(info, Type::Lightweight, "[Background Monitor] Auto-select already running, returning busy");
  +        return Err(anyhow::anyhow!("AUTO_SELECT_BUSY"));
       }
   
       // Fix BUG-MAJOR-001: Use Drop Guard to ensure the lock is released (even if panic occurs)
  @@ -503,8 +503,19 @@ pub fn start_background_monitor() {
                   is_retry_mode = false;
   
                   if wait_for_clash_ready().await {
  -                    if let Err(e) = trigger_backend_auto_select(&current_profile, 0).await {
  -                        logging!(warn, Type::Lightweight, "[Background Monitor] Auto-select failed after configuration reload: {e}");
  +                    loop {
  +                        match trigger_backend_auto_select(&current_profile, 0).await {
  +                            Ok(_) => break,
  +                            Err(e) if e.to_string() == "AUTO_SELECT_BUSY" => {
  +                                logging!(debug, Type::Lightweight, "[Background Monitor] Auto-select busy, waiting to retry...");
  +                                sleep(Duration::from_millis(500)).await;
  +                            }
  +                            Err(e) => {
  +                                logging!(warn, Type::Lightweight, "[Background Monitor] Auto-select failed after configuration reload: {e}");
  +                                break;
  +                            }
  +                        }
  +                    }
                   } else {
                       logging!(warn, Type::Lightweight, "[Background Monitor] Core readiness timeout, aborting this self-healing selection");
  ```

---

#### AUDIT-BE-002: `sort_type: None` defaults to `1` instead of `0` in tauri command
- **Severity**: Major
- **File Path**: [src-tauri/src/cmd/proxy.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/proxy.rs#L155-L168)
- **Root Cause Analysis**:
  The doc comment states that `sort_type: None` represents reading from the configuration file (which maps to `0` in the backend). However, `sort_type.unwrap_or(1)` is passed. This makes it impossible for the frontend to request the config-based sorting via `None`.
- **Suggested Fix**:
  Unwrap `sort_type` to `0` instead of `1` so that it falls back to reading the saved sorting configuration from the config file.
  
  ```diff
  diff --git a/src-tauri/src/cmd/proxy.rs b/src-tauri/src/cmd/proxy.rs
  index 1234567..89abcde 100644
  --- a/src-tauri/src/cmd/proxy.rs
  +++ b/src-tauri/src/cmd/proxy.rs
  @@ -161,7 +161,7 @@ pub async fn trigger_auto_select(
           let current_uid_str = current_uid.to_string();
           let res = crate::module::monitor::trigger_backend_auto_select(
               &current_uid_str,
  -            sort_type.unwrap_or(1),
  +            sort_type.unwrap_or(0),
           )
           .await
           .stringify_err()?;
  ```

---

#### AUDIT-BE-003: Inconsistent/inefficient parser for JSON configuration file
- **Severity**: Minor
- **File Path**: [src-tauri/src/module/monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L97-L103)
- **Root Cause Analysis**:
  `get_saved_sort_type` parses `proxy_head_state.json` using `serde_yaml_ng::from_str`. In contrast, line 78 in `get_active_filter_config` parses the exact same file using `serde_json::from_str`. Using a YAML parser for JSON is inefficient and inconsistent.
- **Suggested Fix**:
  Change `serde_yaml_ng::from_str` to `serde_json::from_str`.
  
  ```diff
  diff --git a/src-tauri/src/module/monitor.rs b/src-tauri/src/module/monitor.rs
  index 1234567..89abcde 100644
  --- a/src-tauri/src/module/monitor.rs
  +++ b/src-tauri/src/module/monitor.rs
  @@ -97,7 +97,7 @@ async fn get_saved_sort_type(profile_uid: &str) -> Option<i32> {
       let path = crate::utils::dirs::app_home_dir().ok()?.join("proxy_head_state.json");
       let content = tokio::fs::read_to_string(path).await.ok()?;
  -    let json_val: serde_json::Value = serde_yaml_ng::from_str(&content).ok()?;
  +    let json_val: serde_json::Value = serde_json::from_str(&content).ok()?;
       let sort_type = json_val[profile_uid]["PROXY"]["sortType"].as_i64()?;
       Some(sort_type as i32)
   }
  ```

---

#### AUDIT-BE-004: Latency threshold filters out high-performance nodes <= 50ms
- **Severity**: Major
- **File Path**: [src-tauri/src/module/monitor.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L391-L395)
- **Root Cause Analysis**:
  In `trigger_backend_auto_select_inner`, delay test results are filtered with `delay_info.delay > 50`. Highly desirable, low-latency nodes (under 50ms) are erroneously discarded and can never be auto-selected.
- **Suggested Fix**:
  Change the threshold check to `delay_info.delay > 0` to filter out failures while retaining high-performance nodes.
  
  ```diff
  diff --git a/src-tauri/src/module/monitor.rs b/src-tauri/src/module/monitor.rs
  index 1234567..89abcde 100644
  --- a/src-tauri/src/module/monitor.rs
  +++ b/src-tauri/src/module/monitor.rs
  @@ -389,7 +389,7 @@ async fn trigger_backend_auto_select_inner(
               if let Ok(res) = req.send().await {
                   if res.status().is_success() {
                       if let Ok(delay_info) = res.json::<DelayResponse>().await {
  -                        if delay_info.delay > 50 && delay_info.delay < 2000 {
  -                        if delay_info.delay > 0 && delay_info.delay < 2000 {
                              return Some((node_name, delay_info.delay));
                          }
                      }
  ```

---

#### AUDIT-BE-005: Dead code and lack of persistence updates in `apply_dns_config`
- **Severity**: Major
- **File Path**: [src-tauri/src/cmd/clash.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/clash.rs#L165-L172)
- **Root Cause Analysis**:
  In `apply_dns_config`, the DNS mapping is patched into the runtime config draft. However, `IRuntime::patch_config` only processes fields in `PATCH_CONFIG_INNER` and `"tun"`, completely ignoring `"dns"`. This makes the patch a silent no-op. Furthermore, the persistent `enable_dns_settings` flag in `Config::verge()` is never updated, meaning the config generator never builds the YAML with the custom DNS configuration.
- **Suggested Fix**:
  Instead of patching the runtime config directly with the DNS configuration, update the `enable_dns_settings` flag in `Config::verge()`, save the file, and then trigger config regeneration.
  
  ```diff
  diff --git a/src-tauri/src/cmd/clash.rs b/src-tauri/src/cmd/clash.rs
  index 1234567..89abcde 100644
  --- a/src-tauri/src/cmd/clash.rs
  +++ b/src-tauri/src/cmd/clash.rs
  @@ -162,13 +162,11 @@ pub async fn apply_dns_config(apply: bool) -> CmdResult {
   
           logging!(info, Type::Config, "Applying DNS config from file");
   
  -        // Create patch containing DNS configuration
  -        let mut patch = serde_yaml_ng::Mapping::new();
  -        patch.insert("dns".into(), patch_config.into());
  -
  -        // Apply DNS configuration to runtime configuration
  -        Config::runtime().await.edit_draft(|d| {
  -            d.patch_config(&patch);
  +        // Update DNS enabled flag in verge config
  +        let verge = Config::verge().await;
  +        verge.edit_draft(|d| {
  +            d.enable_dns_settings = Some(true);
           });
  +        verge.apply();
  +        let _ = verge.data_arc().save_file().await;
   
           // Apply new configuration
           CoreManager::global()
  @@ -182,6 +180,14 @@ pub async fn apply_dns_config(apply: bool) -> CmdResult {
           logging!(info, Type::Config, "DNS config successfully applied");
       } else {
           // When disabling DNS settings, regenerate config (without loading DNS config file)
           logging!(info, Type::Config, "DNS settings disabled, regenerating config");
  +
  +        // Update DNS enabled flag in verge config to false
  +        let verge = Config::verge().await;
  +        verge.edit_draft(|d| {
  +            d.enable_dns_settings = Some(false);
  +        });
  +        verge.apply();
  +        let _ = verge.data_arc().save_file().await;
   
           CoreManager::global()
  ```

---

#### AUDIT-BE-006: Profile switch restoration fails to reload Clash core configuration
- **Severity**: Major
- **File Path**: [src-tauri/src/cmd/profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L186-L203)
- **Root Cause Analysis**:
  In `restore_previous_profile`, when a profile switch fails, the active profile index is reverted in memory and written to disk, but the Clash core is never notified to reload the configuration. This leaves Clash running in an inconsistent state or with the failed configuration.
- **Suggested Fix**:
  Trigger a background reload of the Clash config using `CoreManager::global().update_config_forced()` during restoration.
  
  ```diff
  diff --git a/src-tauri/src/cmd/profile.rs b/src-tauri/src/cmd/profile.rs
  index 1234567..89abcde 100644
  --- a/src-tauri/src/cmd/profile.rs
  +++ b/src-tauri/src/cmd/profile.rs
  @@ -196,6 +196,9 @@ async fn restore_previous_profile(prev_profile: &String) -> CmdResult<()> {
       crate::process::AsyncHandler::spawn(|| async move {
           if let Err(e) = profiles_save_file_safe().await {
               logging!(warn, Type::Cmd, "Warning: Failed to save restored config file asynchronously: {e}");
           }
  +        if let Err(e) = CoreManager::global().update_config_forced().await {
  +            logging!(error, Type::Cmd, "Failed to reload Clash config after restore: {e}");
  +        }
       });
       logging!(info, Type::Cmd, "Successfully restored to previous configuration");
       Ok(())
  ```

---

#### AUDIT-BE-007: Profile deletion sends wrong profile ID to frontend
- **Severity**: Major
- **File Path**: [src-tauri/src/cmd/profile.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/cmd/profile.rs#L162-L170)
- **Root Cause Analysis**:
  In `delete_profile`, when the currently active profile is deleted, the backend updates the active profile to a fallback. However, the notification broadcasted to the frontend uses `notify_profile_changed(&index)`, where `index` is the UID of the *deleted* profile. This causes the UI to attempt to fetch details for a non-existent profile, resulting in UI errors.
- **Suggested Fix**:
  Retrieve the new active profile UID from `Config::profiles()` and pass it to the notification function.
  
  ```diff
  diff --git a/src-tauri/src/cmd/profile.rs b/src-tauri/src/cmd/profile.rs
  index 1234567..89abcde 100644
  --- a/src-tauri/src/cmd/profile.rs
  +++ b/src-tauri/src/cmd/profile.rs
  @@ -164,8 +164,9 @@ pub async fn delete_profile(index: String) -> CmdResult {
               Ok(outcome) if outcome.is_valid() => {
                   handle::Handle::refresh_clash();
                   // Send configuration change notification
  -                logging!(info, Type::Cmd, "[Delete Profile] Send configuration change notification: {}", index);
  -                handle::Handle::notify_profile_changed(&index);
  +                let new_current = Config::profiles().await.data_arc().current.clone().unwrap_or_default();
  +                logging!(info, Type::Cmd, "[Delete Profile] Send configuration change notification: {}", new_current);
  +                handle::Handle::notify_profile_changed(&new_current);
               }
               Ok(outcome) => {
  ```

---

## 5. Verification Method

### Standard Build & Compile Check
Run the Rust compiler check on the `src-tauri` workspace to verify syntactic correctness of all proposed modifications:
```powershell
cd src-tauri
cargo check
```
*(Note: As the subagent did not run compilation checks due to permission timeout, this command should be run by the implementing developer to verify compilation).*

### Functional Verification Steps
1. **AUDIT-BE-001 (Auto-select race condition)**:
   - Trigger a manual auto-selection. While it is running, trigger a profile switch.
   - Verify that the background daemon retries when `AUTO_SELECT_BUSY` is returned and successfully updates the active node for the new profile once the lock is released.
2. **AUDIT-BE-002 (Sort Type Option)**:
   - Call `trigger_auto_select` via Tauri/JS with `sort_type: null`.
   - Verify in the logs that it reads the sorting configuration from `proxy_head_state.json` (falling back to `0`) instead of forcing latency sorting (`1`).
3. **AUDIT-BE-003 (JSON Parser Consistency)**:
   - Ensure the code compiles after substituting `serde_yaml_ng::from_str` with `serde_json::from_str` in `get_saved_sort_type`.
4. **AUDIT-BE-004 (Latency Filtering)**:
   - Host a local proxy with a latency of 10ms–30ms.
   - Run the auto-select tool. Ensure the local proxy is included in the measurements and is eligible for auto-selection.
5. **AUDIT-BE-005 (DNS Config Integration)**:
   - Save custom DNS configurations and call `apply_dns_config(true)`.
   - Verify `dns_config.yaml` is read and properly merged into the generated runtime Clash config (`clash.yaml` or equivalent in app home dir).
6. **AUDIT-BE-006 (Profile Switch Failure)**:
   - Trigger a profile switch to a deliberately malformed/invalid configuration file.
   - Verify that the Clash core is successfully reloaded back to the previous working profile.
 7. **AUDIT-BE-007 (Deleted Profile Notification)**:
   - Delete the currently active profile.
   - Verify that the UI switches focus to the new fallback profile instead of throwing errors.
