# Handoff Report — 2026-06-24T22:37:46+08:00

## 1. Observations
This read-only audit of the Rust backend core logic and configurations under `src-tauri/` in Clash Mini (Clash Verge) identified 8 key findings related to correctness, concurrency, security, and reliability. 

Here are the direct observations from the code:

### Finding 1: Zero-Length Slice Read in HTTP Latency Test (Correctness/Logic Bug)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/feat/clash.rs#L163`
*   **Code Segment**:
    ```rust
    130:         let mut buf = BytesMut::with_capacity(1024);
    ...
    163:             let _ = stream.read(&mut buf).await?;
    ```
*   **Root Cause**: `BytesMut::with_capacity(1024)` creates a buffer with capacity but zero length. When `stream.read(&mut buf)` is called, Rust coerces `&mut buf` into a `&mut [u8]` slice. Because the length of `buf` is 0, the slice passed to `read` is `&mut []`. A read into a zero-length slice returns `Ok(0)` immediately without waiting for data on the socket.

### Finding 2: Concurrency Hang / Infinite Loop in Script Validation (Concurrency Bug)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs#L247`
*   **Code Segment**:
    ```rust
    246:         let mut context = Context::default();
    247:         let result = context.eval(Source::from_bytes(&content));
    ```
*   **Root Cause**: JavaScript script validation (`validate_script_file_outcome`) runs JavaScript code inside `boa_engine` synchronously via `context.eval`. No execution timeout, instruction limit, or thread watchdog is configured.

### Finding 3: Core Validation Infinite Hang due to Missing Command Timeout (Concurrency Bug)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/validate.rs#L362`
*   **Code Segment**:
    ```rust
    362:         let output = command.output().await?;
    ```
*   **Root Cause**: Spawned core validation process via `command.output().await?` has no timeout. If the sub-process hangs (e.g. port conflict, network resolver hang, core bug), `output().await` blocks indefinitely.

### Finding 4: SSRF Bypass via DNS Resolution and IPv6 Local Ranges (Security Vulnerability)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/config/prfitem.rs#L824`
*   **Code Segment**:
    ```rust
    824: fn validate_url_no_ssrf(url: &Url) -> Result<()> {
    825:     if let Some(host) = url.host() {
    ...
    827:             url::Host::Domain(d) => {
    ...
    835:                     bail!("cannot fetch subscription from localhost/loopback address");
    ...
    843:             url::Host::Ipv6(ip) => {
    844:                 if ip.is_loopback() || ip.is_unspecified() {
    845:                     bail!("cannot fetch subscription from loopback IP");
    846:                 }
    ```
*   **Root Cause**: SSRF verification performs string checks on the domain only and omits DNS resolution (allowing domain-based bypasses like local DNS resolving or DNS rebinding). Additionally, for IPv6, it only blocks loopback/unspecified, omitting local-link/ULA IPv6 address ranges (e.g. `fc00::/7` and `fe80::/10`).

### Finding 5: Silent Application Exit on Port Collision (UX/Reliability Bug)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/lib.rs#L234`
*   **Code Segment**:
    ```rust
    234:     if app_init::init_singleton_check().is_err() {
    235:         return;
    236:     }
    ```
*   **Root Cause**: `init_singleton_check` attempts to check if the singleton server port is occupied. If the port is occupied by a non-Clash-Verge process (or connection fails due to network configuration), the HTTP check request fails and returns an `Err`. In `lib.rs`, `run()` exits immediately if `init_singleton_check().is_err()` is true.

### Finding 6: Lost Notification Bug in Background Monitor (Concurrency Bug)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/module/monitor.rs#L437` & `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/core/handle.rs#L51`
*   **Code Segments**:
    ```rust
    // monitor.rs:
    437:                 _ = PROFILE_SWITCH_NOTIFY.notified() => {
    ```
    ```rust
    // handle.rs:
    51:         crate::module::monitor::PROFILE_SWITCH_NOTIFY.notify_waiters();
    ```
*   **Root Cause**: `PROFILE_SWITCH_NOTIFY` is a `tokio::sync::Notify`. When profile changes, `notify_waiters()` is called. `notify_waiters()` only wakes up tasks *currently* waiting on `notified()`. Unlike `notify_one()`, it does NOT store a permit if there are no waiters.

### Finding 7: Crash/Error on Relative Startup Script Paths (Reliability Bug)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/init.rs#L451`
*   **Code Segment**:
    ```rust
    450:     let parent_dir = script_dir.parent();
    451:     let working_dir = parent_dir.unwrap_or_else(|| script_dir.as_ref());
    ```
*   **Root Cause**: If `script_path` is configured as a relative filename with no directory components (e.g. `"script.sh"`), `script_dir.parent()` returns `None`. `working_dir` falls back to `script_dir` which is `"script.sh"`. Calling `.current_dir` with a file path causes `command.output().await` to fail with `std::io::Error`.

### Finding 8: Unbounded Memory Allocation Risk on Subscriptions (Resource Leak/Crash Risk)
*   **File Path**: `file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/src/utils/network.rs#L216`
*   **Code Segment**:
    ```rust
    216:         let body = match response.text().await {
    217:             Ok(text) => text.into(),
    ```
*   **Root Cause**: `response.text().await` reads the entire HTTP body into a `String` without size limitation. If a subscription URL points to a huge file or infinite stream (either maliciously or due to server error), it will allocate memory indefinitely until OOM occurs.

---

## 2. Logic Chain

1.  **Finding 1 (Zero-Length Slice Read)**:
    *   *Observation*: `buf` is initialized as a zero-length `BytesMut` (line 130).
    *   *Observation*: `stream.read(&mut buf)` is called in the HTTP branch (line 163).
    *   *Deduction*: `stream.read` expects `&mut [u8]`. The dereference of `BytesMut` returns a slice of length `self.len()`, which is `0`.
    *   *Conclusion*: The TCP read returns immediately with `Ok(0)`. Therefore, the HTTP latency check completes without waiting for the remote server's response. For proxied connections, it only measures the time to connect to localhost and write the request, rendering all HTTP latency checks incorrect and returning ~0ms RTT.

2.  **Finding 2 (JS Script Validation Hang)**:
    *   *Observation*: JavaScript validation evaluates the script content via `context.eval` (line 247).
    *   *Observation*: No timeout or execution instruction limit is configured for the Boa engine.
    *   *Deduction*: If JS code contains a loop like `while(true) {}`, the thread executing `context.eval` will spin indefinitely.
    *   *Conclusion*: This blocks the tokio runtime thread permanently. The config validator state (`is_processing`) remains locked to `true` via `defer!`, causing all subsequent validation requests to fail with `ValidationOutcome::Busy`.

3.  **Finding 3 (Core Validation process hang)**:
    *   *Observation*: `command.output().await?` is called (line 362) without any tokio timeout.
    *   *Deduction*: If the Mihomo core process blocks (e.g. port conflict or socket hangs), it will never exit.
    *   *Conclusion*: The thread remains blocked forever, locking the config validator in `Busy` state indefinitely.

4.  **Finding 4 (SSRF Bypass)**:
    *   *Observation*: `validate_url_no_ssrf` only performs string checks on the domain (lines 827-834) and blocks loopback/unspecified for IPv6 (lines 843-847).
    *   *Deduction*: No DNS lookup is performed. If a domain name points to `127.0.0.1` (DNS rebinding) or if local link/ULA IPv6 addresses are used (which are not loopback/unspecified), the validator approves the URL.
    *   *Conclusion*: Malicious profiles can easily bypass the SSRF checks and probe the local network.

5.  **Finding 5 (Silent Exit on Port Collision)**:
    *   *Observation*: `run()` terminates immediately if `init_singleton_check()` returns `Err` (lines 234-236).
    *   *Deduction*: If the port is bound by a non-Clash-Verge app or if the connection fails, the HTTP check request fails and returns an `Err`.
    *   *Conclusion*: The application exits silently without any user notification.

6.  **Finding 6 (Lost Notification Bug)**:
    *   *Observation*: `PROFILE_SWITCH_NOTIFY` wakes up waiters using `notify_waiters()` (line 51).
    *   *Observation*: The background monitor loop is blocked on `trigger_backend_auto_select(...).await` (line 485) while doing auto-selection.
    *   *Deduction*: Since the background monitor is not awaiting `PROFILE_SWITCH_NOTIFY.notified()` during the auto-select, `notify_waiters()` wakes up nothing and stores no permit.
    *   *Conclusion*: The notification is lost. The background monitor will wait for the full `check_interval` (up to 15s) to detect a profile switch, leading to a long delay before auto-select starts for the new profile.

7.  **Finding 7 (Relative Startup Script Crash)**:
    *   *Observation*: `parent_dir.unwrap_or_else(|| script_dir.as_ref())` sets `working_dir` to the script file path if no parent exists (line 451).
    *   *Deduction*: Setting a file path as a process's current working directory is invalid.
    *   *Conclusion*: Spawning the process fails with a `std::io::Error`, crashing the script execution.

8.  **Finding 8 (Unbounded Memory Allocation)**:
    *   *Observation*: `response.text().await` reads the entire response into memory (line 216).
    *   *Deduction*: No check on response headers (`Content-Length`) or chunk stream limit is done.
    *   *Conclusion*: Downloading a huge file or infinite stream will cause high memory usage or OOM crashes.

---

## 3. Caveats
- No caveats. The codebase has been audited statically and matches the observations.

---

## 4. Conclusion
While the major Windows named pipe issues (Finding 1) and socket timeout bypasses (Finding 2 & 3) from the previous commit range have been fixed in the latest HEAD, new and critical correctness, security, and concurrency issues exist:
1.  **Correctness**: HTTP latency tests are completely broken, especially when a proxy is enabled (returns ~0ms instantly).
2.  **Concurrency**: Script and core validation are vulnerable to infinite hangs, locking up tokio worker threads and permanently disabling configuration validation. Background monitor notifications can be lost, causing a 15-second delay.
3.  **Security**: SSRF protection is bypassable.
4.  **UX & Reliability**: Port collisions lead to silent exits, relative startup script paths crash execution, and subscriptions lack memory bounds.

---

## 5. Verification Method
- **Static Verification**:
  1. Inspect the source file: `src-tauri/src/feat/clash.rs` at line 163. Observe `buf` is a `BytesMut` with zero length.
  2. Inspect `src-tauri/src/core/validate.rs` at line 247. Note `context.eval` is called without any watchdog/instruction limits.
  3. Inspect `src-tauri/src/core/handle.rs` at line 51 and `src-tauri/src/module/monitor.rs` at line 437. Observe the use of `notify_waiters()` which does not persist permits.
