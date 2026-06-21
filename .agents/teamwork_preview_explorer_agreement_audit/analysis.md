# Clash Mini Codebase Audit Report

## Executive Summary
This report presents the compliance audit of modifications made to the Clash Mini codebase between release commit `d3831a0ce5ecc6b2c040368570773f2622d0b91b` and latest HEAD (`196e7c01`) against the requirements laid out in `clash_mini_agreements.md`. 

Overall, the modifications show excellent adherence to the design principles, theme integrations (MUI styles, 3D button styling, disabled states), and backend socket communication enhancements. We identified a few minor inconsistencies and deviations, primarily involving timeout mismatches and frontend-backend threshold alignment.

---

## 1. Observation

### File-by-File Observations:

#### A. `Cargo.lock`
- **Location**: `Cargo.lock`
- **Content**: Version bump of `clash-mini` package from `1.5.3` to `1.5.4`.
```diff
 [[package]]
 name = "clash-mini"
-version = "1.5.3"
+version = "1.5.4"
 dependencies = [
```

#### B. `crates/tauri-plugin-mihomo/guest-js/index.ts`
- **Location**: `crates/tauri-plugin-mihomo/guest-js/index.ts` (Lines 112-115)
- **Content**: Added deprecation JSDoc comment for the unused `delayGroup` function.
```typescript
 * @deprecated [Clash Mini 备注]: 此接口为 Clash Verge 遗留设计，在 Clash Mini 中前台批量测速已统一收拢到后端 trigger_auto_select，本接口目前无任何地方调用，仅作为 API 完整性保留。
```

#### C. `crates/tauri-plugin-mihomo/src/commands.rs`
- **Location**: `crates/tauri-plugin-mihomo/src/commands.rs` (Lines 70-73)
- **Content**: Added comment clarifying that the `delay_group` command is a remnant of Clash Verge.
```rust
// [Clash Mini 备注]: 此命令为 Clash Verge 遗留接口。
// 在 Clash Mini 中，前台批量测速已统一收拢到后端的 trigger_auto_select 触发并返回结果，
// 此处在 Clash Mini 内属于闲置死代码，保留仅为了维持插件 API 接口的完整性。
```

#### D. `crates/tauri-plugin-mihomo/src/mihomo.rs`
- **Location**: `crates/tauri-plugin-mihomo/src/mihomo.rs` (Lines 756-758)
- **Content**: Added deprecation comment for the Rust implementation of `delay_group`.
```rust
    /// 对指定代理组进行延迟测试, 同时清理代理组已固定的节点
    /// [Clash Mini 备注]: 此方法对应 Clash Verge 的策略组延迟测速，目前在 Clash Mini 内已无任何调用，保留仅作为备用和对齐内核 API。
```

#### E. `src-tauri/src/module/monitor.rs`
- **Location**: `src-tauri/src/module/monitor.rs` (Multiple lines)
- **Content**:
  - Removed HTTP-based client (`reqwest::Client`) and its associated data types (`ProxyGroupInfo`, `DelayResponse`).
  - Switched to the `Mihomo` local socket client instance via `crate::core::handle::Handle::mihomo().await`.
  - `wait_for_clash_ready` checks:
    - Phase 1: `mihomo.get_base_config().await.is_ok()` (Lines 131-132)
    - Phase 2: `mihomo.get_group_by_name("PROXY").await` (Lines 148-150)
  - `check_active_node_health` checks:
    - `mihomo.delay_proxy_by_name(active_node, test_url, 500).await` (Line 197) and determines health via `if delay_info.delay > 0` (Line 198).
  - `trigger_backend_auto_select_inner` checks:
    - `mihomo.delay_proxy_by_name(&node_name, &test_url, 2000).await` (Line 301) and filters via `if delay_info.delay > 0 && delay_info.delay < 2000` (Line 302).
    - Limit concurrency via `Semaphore::new(MAX_CONCURRENT_DELAY_TESTS)` where `MAX_CONCURRENT_DELAY_TESTS = 32`.
    - Auto-select loop in `start_background_monitor` handles intervals of `3` seconds (failure mode) or `15` seconds (normal mode) based on `check_interval = if is_retry_mode { 3 } else { 15 }` (Line 422).
    - Failure threshold is `consecutive_fails >= 3` (Line 441).

#### F. `src/pages/_layout.tsx`
- **Location**: `src/pages/_layout.tsx` (Lines 215-277, 340-343, 357, 380)
- **Content**:
  - Updated `frontendAutoSelect`:
    - Triggers `DelayManager.checkListDelay(allNames, groupName, timeout, concurrency)` asynchronously (Line 38).
    - Implemented **Flash-connect** (临时闪连): Once `healthyNodes.length >= 1`, performs `selectNodeForGroup(groupName, tempTarget)` and refreshes display (Lines 87-97).
    - Implemented **Early-termination** (极速终选与提前终止): Triggers final choice when `healthyNodes.length >= 5 || testedCount >= allNames.length || elapsed >= 15000` (Lines 104-107).
    - Cap concurrency at `36` (Line 22, 141).
  - Mismatch:
    - Line 340: `// 协议要求：6 秒无健康节点 → Fallback 降级，强制全节点测速`
    - Line 380: `}, 10000)` (Timeout set to 10 seconds).
    - Line 357: `const hasHealth = latestDelay > 50 && latestDelay < 2000` (Filters out delay < 50ms).

#### G. `src/services/delay.ts`
- **Location**: `src/services/delay.ts` (Lines 265-269)
- **Content**:
  - Moved the UI sweep/glow notification `queueGroupNotification` before initiating the async concurrent test workers.
```typescript
    const listener = this.groupListenerMap.get(group)
    // 瞬间通知 UI 全组开始测速扫光
    if (listener) {
      this.queueGroupNotification(group)
    }
```

#### H. `src/utils/button-styles.ts`
- **Location**: `src/utils/button-styles.ts` (Multiple themes inside `get3DButtonStyle`)
- **Content**:
  - Added specific `&.Mui-disabled` styling overrides for all skins (`retro-3d`, `original`, `modern-flat`, `frosted-glass`, `cyberpunk`, `monochrome`).
  - These use `!important` to force colors, borders, and backgrounds, disabling animations/shadows/transforms when buttons are disabled.
```typescript
      '&.Mui-disabled': {
        background: isLight ? '#e0e0e0 !important' : '#32323a !important',
        borderColor: isLight ? '#bdbdbd !important' : '#4a4a52 !important',
        color: isLight ? '#9e9e9e !important' : '#6e6e75 !important',
        boxShadow: 'none !important',
        textShadow: 'none !important',
        transform: 'none !important',
      },
```

---

## 2. Logic Chain

1. **Inconsistency in Fallback Timeout**:
   - **Observation**: In `_layout.tsx`, the comment on line 340 says: `// 协议要求：6 秒无健康节点 → Fallback 降级，强制全节点测速`. However, the `setTimeout` on line 380 is set to `10000` (10 seconds).
   - **Agreement Reference**: Section III.7 / BUG-053 specifies: "如果自动测速已启动且轮询时间超过 6 秒 依然未扫描到任何健康可用节点，后台会自动向前端发送直接延迟测速命令（checkListDelay）作为 fallback 机制...".
   - **Inference**: The implementation deviates from both the code comment and the strict 6-second specification of the agreement, resulting in a delayed fallback (10 seconds).

2. **Healthy Node Delay Threshold Discrepancy**:
   - **Observation**: Backend `monitor.rs` defines a healthy node as having `delay > 0` (Line 198) and `delay > 0 && delay < 2000` (Line 302). However, frontend `_layout.tsx` defines a healthy node as having `delay >= 50 && delay < timeout` (Line 64) and `latestDelay > 50 && latestDelay < 2000` (Line 357).
   - **Agreement Reference**: General latency check guidelines.
   - **Inference**: A node with a delay between 1ms and 49ms is classified as healthy by the backend monitor (meaning it will remain selected and not trigger auto-select healing) but classified as unhealthy by the frontend (meaning it will trigger the fallback check or be ignored in the UI's health calculations). This mismatch causes inconsistent state evaluations between the backend daemon and the frontend UI.

3. **Backend Local Socket Timeout Deviation**:
   - **Observation**: Backend `monitor.rs` initiates local socket requests such as `get_base_config` (Line 132) and `get_group_by_name` (Line 148), which do not explicitly override timeouts and thus fall back to `DEFAULT_REQUEST_TIMEOUT = 5 seconds` (defined in `mihomo.rs`). For `delay_proxy_by_name`, it sets `req_timeout = timeout + DEFAULT_REQUEST_TIMEOUT`, which is `2000ms + 5s = 7 seconds` (Line 301).
   - **Agreement Reference**: Section II.25 / BUG-083/093 specifies: "所有的后端监控客户端以及 Local Socket 请求必须配置 3秒 的显式超时限制...".
   - **Inference**: Allowing Local Socket calls to hang for 5 to 7 seconds violates the 3-second limit agreement, potentially blocking the async background monitor thread longer than intended if the Clash Core is unresponsive.

---

## 3. Caveats
- No compile/run-time testing was performed since this is a read-only investigation.
- It is assumed that the `DEFAULT_REQUEST_TIMEOUT` in `crates/tauri-plugin-mihomo/src/mihomo.rs` cannot be easily modified without affecting other parts of the Tauri plug-in (hence it is safer to override timeouts per-request in `monitor.rs` where necessary).

---

## 4. Conclusion
The modifications generally fulfill the agreement requirements:
- **36-concurrency limit** is correctly enforced in `_layout.tsx` (using 36) and `monitor.rs` (using 32).
- **Flash-connect** and **Early-termination** are fully implemented in `_layout.tsx`.
- **Disabled button text contrast rules** are implemented in `button-styles.ts` across all 6 skins.
- **UI sweep/glow animation feedback** starts instantly in `delay.ts`.
- **Backend monitor** uses Local Socket and adheres to the 15s/3s checks and 3-fail threshold.

However, three compliance issues remain:
1. **Fallback Timeout Inconsistency** in `_layout.tsx` (10 seconds used instead of 6 seconds).
2. **Healthy Node Delay Threshold Mismatch** between backend (`delay > 0`) and frontend (`delay >= 50`).
3. **Rust Local Socket Timeout Mismatch** (using 5-7s timeouts instead of the 3s limit).

---

## 5. Verification Method

To verify the audit findings:
1. Inspect `src/pages/_layout.tsx` at lines 340-343 and 380 to verify the `setTimeout` delay is `10000` (10s) instead of `6000` (6s).
2. Compare `src-tauri/src/module/monitor.rs` (Lines 198, 302) with `src/pages/_layout.tsx` (Lines 64, 357) to verify the `> 0` vs `>= 50` threshold difference.
3. Compare `crates/tauri-plugin-mihomo/src/mihomo.rs` (Line 32) and `src-tauri/src/module/monitor.rs` to verify that local socket queries do not override their default timeout (resulting in a 5s default timeout and 7s delay test timeout).

---

## 6. Proposed Code Fixes / Recommendations

### Recommendation A: Fix Fallback Timeout in Frontend
In `src/pages/_layout.tsx`, update the fallback timer from 10 seconds to 6 seconds:
```diff
<<<<
  // 协议要求：6 秒无健康节点 → Fallback 降级，强制全节点测速
  // 先清理旧定时器，防止重复
  if (fallbackTimerRef.current) clearTimeout(fallbackTimerRef.current)
  fallbackTimerRef.current = setTimeout(async () => {
    ...
  }, 10000)
====
  // 协议要求：6 秒无健康节点 → Fallback 降级，强制全节点测速
  // 先清理旧定时器，防止重复
  if (fallbackTimerRef.current) clearTimeout(fallbackTimerRef.current)
  fallbackTimerRef.current = setTimeout(async () => {
    ...
  }, 6000)
>>>>
```

### Recommendation B: Align Healthy Delay Thresholds
Either lower the frontend threshold to `>= 0` (or `> 0`) to match the backend, or raise the backend threshold to `>= 50` in `monitor.rs`.
If we raise the backend threshold in `src-tauri/src/module/monitor.rs`:
```diff
<<<<
    if let Ok(delay_info) = mihomo.delay_proxy_by_name(active_node, test_url, 500).await {
        if delay_info.delay > 0 {
            // 只要能在 500ms 内成功获取延迟，即判定为健康
            return Ok(true);
        }
    }
====
    if let Ok(delay_info) = mihomo.delay_proxy_by_name(active_node, test_url, 500).await {
        if delay_info.delay >= 50 {
            // 只要能在 500ms 内成功获取延迟，且不低于 50ms，即判定为健康
            return Ok(true);
        }
    }
>>>>
```
And similarly in `trigger_backend_auto_select_inner` (Line 302):
```diff
<<<<
                if delay_info.delay > 0 && delay_info.delay < 2000 {
                    return Some((node_name, delay_info.delay));
                }
====
                if delay_info.delay >= 50 && delay_info.delay < 2000 {
                    return Some((node_name, delay_info.delay));
                }
>>>>
```

### Recommendation C: Configure Explicit 3s Timeout for Backend Socket Requests
Modify the backend monitor requests or plug-in implementation to enforce a strict 3-second timeout limit for regular commands.
In `crates/tauri-plugin-mihomo/src/mihomo.rs`, change the default request timeout to 3 seconds:
```diff
<<<<
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
====
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
>>>>
```
And adjust `delay_proxy_by_name` (Line 941) so that the total timeout does not exceed 3 seconds when performing background health tests.
