# Clash Mini 第三方代码审计报告 (Clash Mini Third-Party Code Audit Report)

## ✍️ 执行摘要 (Executive Summary)
本报告是由独立第三方审计团队针对 **Clash Mini** 应用程序（包含 React/TypeScript 前端与 Rust/Tauri 后端）进行的全面静态代码审计。本审计旨在排查安全隐患、性能故障，并评估代码整洁度与软件架构设计。

### 📊 整体代码库质量综合评价与架构健康度打分
*   **整体架构健康度得分**：**86 / 100**
*   **主要优势**：
    *   **协议执行度极高**：项目高度契合 `clash_mini_agreements.md` 中的绝大多数规定（如端口规避、运行期系统隔离、管理员模式泄漏规避等）。
    *   **后端鲁棒性较好**：Rust 代码安全处理较为扎实，很少使用 `unwrap` 或 `expect` 等可能引起崩溃的硬编码方法，且使用了饱和数学运算保护。
    *   **前端状态管理清晰**：前端使用 `foxact` 实现了精简 of 上下文状态传递。
*   **核心改进空间**：
    *   **并发与锁安全**：后端在多个异步 yields 点跨越持有 `RwLock` 读锁，存在潜在的死锁和延迟抖动风险。
    *   **线程调度阻塞**：后端在 Tokio 异步工作线程上频繁调用同步文件 I/O、同步系统进程扫描（`sysinfo`）等阻塞调用；在 Tauri 启动 setup 钩子中执行同步 `block_on`，阻塞了主 UI 线程。
    *   **前端渲染性能**：前端存在由于 `useWindowWidth` 在 resize 事件中像素级触发多组件更新导致的重绘风暴，以及 Tauri 事件监听器在组件卸载后未正确清理带来的内存泄漏隐患。
    *   **架构过度耦合**：前端 `_layout.tsx` 变为了一个近 5000 行、拥有 38 个 `useState` 的巨型“上帝组件”；同时全局类型大量积压于 `global.d.ts` 命名空间中，缺少模块化导出。

---

## 🔒 第一部分：安全与性能类审计 (Safety & Performance)

### 1. 异步事件监听器内存泄漏 (Tauri Event Listener Leak)
*   **具体文件路径**：`src/providers/app-data-provider.tsx`
*   **受影响代码行范围**：230–292
*   **成因分析**：
    在 `useEffect` 中注册 Tauri 事件监听器（`profile-changed` 和 `verge://refresh-proxy-config`）时，其注册操作是异步的（使用 `await listen(...)`）。当组件在这些 Promise 还没 resolve 之前就被快速卸载（例如由于路由跳转或状态改变），同步的 `useEffect` 清理函数会率先运行（此时 `cleanupFns` 仍为空）。随后 Promise resolve 时，所产生的 `unlisten` 回调函数会被推入 `cleanupFns`，但由于清理阶段已过，它们将永远不会被调用，导致 Tauri 全局事件监听器持续驻留在内存中，形成累积性内存泄漏。
*   **相关代码片段**：
    ```typescript
    useEffect(() => {
      let lastProfileId: string | null = null
      let lastUpdateTime = 0
      const refreshThrottle = 800
      const cleanupFns: Array<() => void> = []

      // ...

      const initializeListeners = async () => {
        try {
          const unlistenProfile = await listen<string>(
            'profile-changed',
            handleProfileChanged,
          )
          cleanupFns.push(unlistenProfile)
        } catch (error) {
          console.error('[AppDataProvider] 监听 Profile 事件失败:', error)
        }

        try {
          const unlistenProxy = await listen(
            'verge://refresh-proxy-config',
            handleRefreshProxy,
          )
          cleanupFns.push(unlistenProxy)
        } catch (error) {
          console.warn('[AppDataProvider] 设置 Tauri 事件监听器失败:', error)
        }
      }

      void initializeListeners()

      return () => {
        cleanupFns.forEach((fn) => {
          try {
            fn()
          } catch (error) {
            console.error('[DataProvider] Cleanup error:', error)
          }
        })
      }
    }, [refreshProxy, refreshRules, refreshRuleProviders])
    ```
*   **具体修复与优化建议**：
    引入布尔值标记 `active` 追踪当前 effect 生命期。若在异步监听注册完成时 effect 已被销毁，则立即调用 unlisten 撤销监听，并用单独的局部变量保存 unlisten 方法。
    ```typescript
    useEffect(() => {
      let active = true
      let unlistenProfile: (() => void) | null = null
      let unlistenProxy: (() => void) | null = null

      const initializeListeners = async () => {
        try {
          const uProfile = await listen<string>('profile-changed', handleProfileChanged)
          if (!active) {
            uProfile()
          } else {
            unlistenProfile = uProfile
          }
        } catch (error) {
          console.error('[AppDataProvider] 监听 Profile 事件失败:', error)
        }

        try {
          const uProxy = await listen('verge://refresh-proxy-config', handleRefreshProxy)
          if (!active) {
            uProxy()
          } else {
            unlistenProxy = uProxy
          }
        } catch (error) {
          console.warn('[AppDataProvider] 设置 Tauri 事件监听器失败:', error)
        }
      }

      void initializeListeners()

      return () => {
        active = false
        if (unlistenProfile) unlistenProfile()
        if (unlistenProxy) unlistenProxy()
      }
    }, [refreshProxy, refreshRules, refreshRuleProviders])
    ```

---

### 2. 窗口缩放高频渲染风暴 (Window Resize Rendering Storm)
*   **具体文件路径**：`src/components/proxy/use-window-width.ts` (1-17行) 及 `src/components/proxy/proxy-item.tsx` (80-82行)
*   **成因分析**：
    `useWindowWidth` 在浏览器 `resize` 事件中像素级地更新数值状态 `width`。而项目中可能有成百上千个代理节点（`ProxyItem`），每个节点都独立调用了 `useWindowWidth()`。当用户拖拽窗口边缘时：
    1. 会产生数百个并发的 window 监听函数同时执行。
    2. 数百个组件独立发生数值级的 `useState` 状态更新，瞬间引发 React 的渲染树重绘风暴，造成显著的 CPU 占用与界面卡顿。
    实际上，`ProxyItem` 仅仅需要知道宽度是否小于等于临界点 `285px` 以切换极简模式，并不需要像素级的精确宽度。
*   **相关代码片段**：
    ```typescript
    // use-window-width.ts
    export const useWindowWidth = () => {
      const [width, setWidth] = useState(() => document.body.clientWidth)

      useEffect(() => {
        const handleResize = () => setWidth(document.body.clientWidth)

        window.addEventListener('resize', handleResize)
        return () => {
          window.removeEventListener('resize', handleResize)
        }
      }, [])

      return { width }
    }

    // proxy-item.tsx
    const { width } = useWindowWidth()
    const isMinimal = width <= 285
    ```
*   **具体修复与优化建议**：
    由于项目使用了 MUI，应直接替换为基于 CSS 媒体查询优化的 MUI 系统 hook，或者编写只监听布尔边界值的自定义 Hook，减少无意义的状态变更触发：
    *   **修复方案 A (MUI 媒体查询)**：
        ```typescript
        import { useMediaQuery } from '@mui/material'
        // 在 ProxyItem 中直接使用：
        const isMinimal = useMediaQuery('(max-width:285px)')
        ```
    *   **修复方案 B (布尔防噪自定义 Hook)**：
        ```typescript
        export const useIsMinimal = (threshold = 285) => {
          const [isMinimal, setIsMinimal] = useState(() => window.innerWidth <= threshold)
          useEffect(() => {
            const handleResize = () => {
              setIsMinimal(window.innerWidth <= threshold)
            }
            window.addEventListener('resize', handleResize)
            return () => window.removeEventListener('resize', handleResize)
          }, [threshold])
          return isMinimal
        }
        ```

---

### 3. 异步跨越持有 `RwLock` 读锁引发死锁 (RwLock Guard across Await)
*   **具体文件路径**：`src-tauri/src/utils/connections_stream.rs` (80-90行)，以及 `core/manager/config.rs:144`、`feat/clash.rs:77`、`feat/window.rs:79`、`utils/connections_stream.rs:150` 等
*   **成因分析**：
    `handle::Handle::mihomo()` 接口返回一个全局 `RwLockReadGuard<'static, Mihomo>` 的 Future。在业务逻辑中，代码在链式调用异步方法（如 `.ws_traffic().await`）时，该临时读锁在整个表达式计算完毕之前不会被 Drop。这导致 **`RwLockReadGuard` 跨越了异步 Yield 点 (`.await`)**。
    在 Tokio 的异步任务调度中，若有另一个写锁请求（例如在配置重载、服务热重启时调用 `.write().await`）在此时排队，该写请求会被阻塞。由于写者优先/防饥饿机制，所有随后的读锁请求也将被挂起。这会导致所有获取 `mihomo()` 的后台测速和流量统计连接大面积挂起死锁。
*   **相关代码片段**：
    ```rust
    // src/utils/connections_stream.rs
    let connection_id = handle::Handle::mihomo()
        .await
        .ws_traffic({
            let message_tx = message_tx.clone();
            move |message| {
                if let Some(event) = parse_traffic_event(&message) {
                    try_send_internal_event(&message_tx, event);
                }
            }
        })
        .await?;
    ```
*   **具体修复与优化建议**：
    必须解除链式调用。在独立作用域块中获取方法生成的 Future，利用局部变量生命周期自动 Drop 掉 `RwLockReadGuard`，之后再对生成的 Future 进行 `.await`：
    ```rust
    let ws_future = {
        let mihomo = handle::Handle::mihomo().await;
        mihomo.ws_traffic({
            let message_tx = message_tx.clone();
            move |message| {
                if let Some(event) = parse_traffic_event(&message) {
                    try_send_internal_event(&message_tx, event);
                }
            }
        })
    }; // 临时变量 mihomo (即 RwLockReadGuard) 在此作用域结束时被 drop
    let connection_id = ws_future.await?;
    ```

---

### 4. 异步运行时工作线程被同步磁盘 I/O 阻塞 (Sync File I/O on Async Workers)
*   **具体文件路径**：`src-tauri/src/module/monitor.rs` (276行) 及 `src-tauri/src/core/updater.rs` (475行)
*   **成因分析**：
    `trigger_backend_auto_select` (监控线程) 和 `check_and_download` (更新下载线程) 均是 Tokio 异步上下文环境。
    1. 在 `monitor.rs` 中，`get_active_filter_config` 频繁通过同步方法 `std::fs::read_to_string` 读取物理磁盘上的 `proxy_head_state.json`。
    2. 在 `updater.rs` 中，静默更新写入缓存文件时使用同步方法 `std::fs::write` 写入下载的二进制更新包（可能达数拾兆大小）。
    这些阻塞 I/O 操作直接在 Tokio 默认工作线程上执行，阻止了该线程的事件调度器轮询其他就绪的 Future，从而间接导致整个客户端的网络通信延迟突变和界面轻微假死。
*   **相关代码片段**：
    ```rust
    // src/module/monitor.rs
    let filter_config = get_active_filter_config(profile_uid);

    // src/core/updater.rs
    if let Err(e) = Self::write_cache(&bytes, &version) {
        logging!(warn, Type::System, "Silent updater: failed to write cache: {e}");
    }
    ```
*   **具体修复与优化建议**：
    1. 针对配置文件读取，使用异步的 `tokio::fs` 代替 `std::fs`；
    2. 针对大容量更新包的磁盘写入操作，使用 `tokio::task::spawn_blocking` 派发给专门的阻塞线程池处理。
    ```rust
    // 写入包方案：
    let version_clone = version.clone();
    let bytes_clone = bytes.clone();
    tokio::task::spawn_blocking(move || {
        Self::write_cache(&bytes_clone, &version_clone)
    }).await.unwrap_or_else(|_| Err(anyhow::anyhow!("Spawn blocking failed")))?;
    ```

---

### 5. 同步系统进程扫描阻塞 (Sync Process Scan blocking Tokio Workers)
*   **具体文件路径**：`src-tauri/src/core/manager/state.rs`
*   **受影响代码行范围**：143-164
*   **成因分析**：
    `CoreManager::kill_all_mini_cores` 依赖于外部库 `sysinfo` 提供的进程检索方法 `sysinfo::System::new_all()`。进程列表扫描是一个高耗时且不可控的系统调用（可能耗时数百毫秒）。此方法在异步核心退出 `stop_core_by_sidecar` 和窗口关闭逻辑中被直接同步调用，导致运行此代码的异步工作线程被死死卡住。
*   **相关代码片段**：
    ```rust
    pub fn kill_all_mini_cores() {
        logging!(
            info,
            Type::Core,
            "Scanning and killing leftover mini-mihomo processes..."
        );
        let system = sysinfo::System::new_all();
        // 随后遍历进程名包含 mini-mihomo 并将其 kill 掉
    ```
*   **具体修复与优化建议**：
    应将此类阻塞的系统进程清单检索动作包裹在 `tokio::task::spawn_blocking` 中执行：
    ```rust
    pub async fn kill_all_mini_cores_async() {
        let _ = tokio::task::spawn_blocking(|| {
            Self::kill_all_mini_cores();
        }).await;
    }
    ```

---

### 6. Tauri Setup 钩子同步阻塞主 UI 线程 (Main Thread block_on in Setup)
*   **具体文件路径**：`src-tauri/src/lib.rs`
*   **受影响代码行范围**：256-260
*   **成因分析**：
    In the Tauri 构建初始化 `.setup()` 钩子中，使用了 `tauri::async_runtime::block_on` 宏阻断执行 `try_install_on_startup` 流程。
    `.setup()` 本身在操作系统的主线程（主 UI 线程与消息循环线程）中执行。在其内部使用 `block_on` 会直接导致应用初始化进程卡死。如果启动时更新模块超时或因网络异常等待超时（默认超时达 30 秒），由于此时 Tauri 自身的原生主事件消息循环尚未激活运转，界面会出现白屏卡死现象。
*   **相关代码片段**：
    ```rust
    let is_updating = tauri::async_runtime::block_on(async {
        crate::core::updater::SilentUpdater::global()
            .try_install_on_startup(&app_handle)
            .await
    });
    ```
*   **具体修复与优化建议**：
    不要阻塞生命周期 Setup 步骤。在 Setup 内部仅以异步任务方式派发更新检测，将控制权立即交还给消息循环；或者在真正的 Tauri 主程序启动前，通过一个小体积的引导装载器 (Bootstrapper) 来提前静默安装更新。
    ```rust
    // 异步派发：
    let app_handle_clone = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        crate::core::updater::SilentUpdater::global()
            .try_install_on_startup(&app_handle_clone)
            .await;
    });
    ```

---

## 🎨 第二部分：代码整洁与架构类审计 (Readability & Architecture)

### 1. 布局文件的上帝组件单体化 (God Component in Layout)
*   **具体文件路径**：`src/pages/_layout.tsx`
*   **受影响代码行范围**：1–4997 (近5000行)
*   **成因分析**：
    `Layout` 组件几乎合并了所有的抽屉侧边栏渲染、连接明细表格、配置文件卡片交互、系统参数控制面板、应用更新对话框等业务逻辑。在组件声明内定义了多达 **38 个独立的 React `useState` 钩子**，严重违背了“单一职责原则 (Single Responsibility Principle)”。当这些状态中的任何一个被微调，React 都需要重新评估这个包含了近 5000 行 JSX 和内部函数的庞大对象，使得代码阅读、排错、后期重构维护变得极为困难。
*   **具体修复与优化建议**：
    进行深度组件剥离：
    1. 将各类面板拆分成独立的子组件放置到 `src/components/` (例如 `ProfilesPanel.tsx`, `ConnectionsPanel.tsx`, `SettingsPanel.tsx`)。
    2. 将更新检测、通知流等逻辑从 Layout 中提炼至自定义 hooks (如 `useAppUpdater.ts`)。
    3. `_layout.tsx` 应当作为纯粹的导航框架结构载体。

---

### 2. 全局环境类型污染 (Global Ambient Type Pollution)
*   **具体文件路径**：`src/types/global.d.ts`
*   **受影响代码行范围**：1–1097
*   **成因分析**：
    一个单独的 `global.d.ts` 环境声明文件打包了项目中绝大多数核心业务领域接口（如 `IConfigData`, `IProxyItem`, `IProxyGroupItem` 等）。全局命名空间污染隐匿了模块依赖的直接引用路径，难以利用 IDE 的重构功能追踪特定数据模型的静态依赖，并极易在未来引入其他模块或进行微前端改造时引发类型定义冲突。
*   **具体修复与优化建议**：
    重构为模块化导出规范。在对应的业务分类目录下存放局部类型（例如 `src/types/clash.ts`, `src/types/profile.ts`），采用 `export interface` 的方式声明，使用时显式 `import` 导入。

---

### 3. 文件命名约定不一致 (Hook File Naming Inconsistency)
*   **具体文件路径**：`src/hooks/useWindowSnap.ts`
*   **成因分析**：
    该文件使用了小驼峰命名（`useWindowSnap.ts`），而 `src/hooks/` 目录下的其他所有自定义 Hook 文件均一贯地使用了中划线分割命名的约定（如 `use-clash.ts`, `use-traffic-monitor.ts`）。
*   **具体修复与优化建议**：
    将文件重命名为 `use-window-snap.ts`，并同步修改各导入路径。

---

### 4. 辅助性 Hook 内部函数缺少 Memoize (Non-Memoized Hook Callbacks)
*   **具体文件路径**：`src/hooks/use-clash.ts` 及 `src/hooks/use-profiles.ts`
*   **成因分析**：
    自定义 Hook 返回的方法（例如 `mutateClash`, `mutateProfiles`, `patchProfiles`）在每次 Hook 重新执行时均会创建新的引用。这导致在子组件中使用这些回调函数并作为 prop 传递或列入 `useEffect` 的 dependency 列表时，由于引用不断更改，容易引发 React 子组件的无效重复渲染，甚至可能诱发无线刷新死循环。
*   **具体修复与优化建议**：
    使用 React `useCallback` 对这些方法进行包裹处理：
    ```typescript
    const mutateProfiles = useCallback(async () => {
      await refetch()
    }, [refetch])
    ```

---

### 5. 宽松的 any 类型定义隐患 (Loose any Types)
*   **具体文件路径**：`src/providers/app-data-context.ts` (11, 14, 44, 60行) 以及 `src/pages/_layout.tsx` (1314, 1324行)
*   **成因分析**：
    应用上下文中一些高频核心变量（如 `proxies` 和 `sysproxy`）以及 Layout 的更新状态变量被直接定义为宽泛的 `any`。这极大降低了编译期的 TypeScript 类型守卫能力，一旦底层 IPC 数据模型更改，会导致前端抛出不可靠的 `Cannot read property of undefined` 运行时错误。
*   **具体修复与优化建议**：
    建立严格的静态类型定义，用具体的 interface 结构定义来替代 `any`（可使用 Tauri 插件 API 输出的契约结构）。

---

### 6. 后端时间戳数值强转截断隐患 (Timestamp Integer Truncation)
*   **具体文件路径**：`src-tauri/src/config/prfitem.rs` (249, 341, 432 等行)
*   **成因分析**：
    后端将 chrono 库生成的 64位整型时间戳（`timestamp()` 返回 `i64`）通过 `as usize` 直接强制转换并存储在 `PrfItem::updated` 等结构体字段中。在 32 位机器或嵌入式架构上运行该客户端时，`usize` 宽度仅为 32 位，这直接产生了 **数值截断**，会引发严重的“2038年问题”或时间戳计算溢出崩溃。
*   **相关代码片段**：
    ```rust
    updated: Some(chrono::Local::now().timestamp() as usize),
    ```
*   **具体修复与优化建议**：
    调整 `PrfItem` 和 `IProfiles` 的相应配置定义为 `Option<i64>` 或 `Option<u64>`，移除 `as usize` 截断转换操作：
    ```rust
    updated: Some(chrono::Local::now().timestamp()),
    ```

---

## ⚖️ 第三部分：协议合规性核对 (Agreement Compliance)

审计团队将 Clash Mini 的实际代码实现与 `clash_mini_agreements.md` 中的所有 26 条核心开发协议进行了详细对照，具体合规性审计结果列表如下：

### 📋 协议合规性审计列表

| 协议条款编号 & 对应 BUG | 功能/设计规范要求描述 | 实际代码实现路径 | 审计合规性判定 | 说明与备注 |
| :--- | :--- | :--- | :--- | :--- |
| **一 (1) / Project Separation** | 客户端定位与数据、端口、Singleton 全面隔离隔离，防误杀 | `src-tauri/tauri.conf.json` lines 16, 29-30<br>`src-tauri/src/constants.rs` lines 4, 11, 15-18<br>`src-tauri/src/utils/dirs.rs` lines 12, 17 | **完全符合 (Fully Compliant)** | 实现了 io.github.clash-mini.clash-mini 独立运行隔离区。 |
| **二 (2) / 3D Skeuomorphic** | 3D 拟物化与物理质感美学，横向切换滑块 | `src/assets/styles/layout.scss`<br>`src/utils/button-styles.ts` | **完全符合 (Fully Compliant)** | 按协议要求提供了包含 Bevel 厚底、金属质感 HSL 渐变的 skeuomorphic 皮肤样式。 |
| **三 (3) / Manual Control** | 手动路径控制，MATCH 全局分流，连接列表精简 | `src/pages/_layout.tsx` lines 2132-2160, 3220-3240<br>`src/services/cmds.ts` lines 18-91 | **完全符合 (Fully Compliant)** | 提供了 3D 双层常驻面板，自动为订阅附加 MATCH 备用出口。 |
| **四 (4) / Tray Icon** | 系统托盘图标与动态刷新 | `src-tauri/src/core/tray/mod.rs` lines 56-57 | **部分符合 (Partially Compliant)** | 托盘图标当前固定为 static png，属于针对 BUG-073 的架构规避设计。 |
| **五 (5) / Copyright** | 版权与二次开发确权声明 | `src/pages/_layout.tsx` lines 3843-3860 | **完全符合 (Fully Compliant)** | 设置抽屉底部已渲染 `© 2026 秋雨潇潇 (修改部分)`。 |
| **六 (6) / Silence** | 静默后台，页面隐藏或关闭时停止 WS 轮询 | `src/hooks/use-traffic-data.ts` lines 36, 43<br>`src/hooks/use-connection-data.ts` lines 30, 36 | **完全符合 (Fully Compliant)** | 通过 visibility 监听状态自动关闭 WS 流量及连接侦听器。 |
| **七 (7) / BUG-057** | 导入新订阅后就绪等待防空白 | `src/pages/_layout.tsx` lines 1913-1935, 1974-1979 | **完全符合 (Fully Compliant)** | 使用 `lastEnhancedProfileRef` 规避了订阅加载未完成前的重复初始化。 |
| **八 (8) / Proxy Stream** | 代理组精简，去掉原版复杂组选择 | `src/services/cmds.ts` lines 168-271 | **完全符合 (Fully Compliant)** | 代码仅支持渲染并拦截单层 PROXY 组出口。 |
| **九 (9) / Sliders Fix** | 滑动输入范围 0-5，步长 0.1 等调节功能 | `src/pages/_layout.tsx` lines 3728, 3780 | **完全符合 (Fully Compliant)** | 对应控制面板滑块已修复，支持浮点参数设定。 |
| **十 (10) / Cyberpunk** | 浅色模式可读性与 Monochrome 尺寸 | `src/assets/styles/layout.scss`<br>`src/components/base/base-switch.tsx` | **完全符合 (Fully Compliant)** | Cyberpunk 样式文字高对比优化，Monochrome 开关变更为 56x28px 规格。 |
| **十一 (11) / BUG-070** | 管理员模式进程防双开 | `src-tauri/src/core/manager/lifecycle.rs` lines 95-98 | **完全符合 (Fully Compliant)** | 管理员启动时自动跳过冗余 sidecar 检测流程，规避冲突双开进程。 |
| **十二 (12) / BUG-071** | 下拉 Help 按钮与手动内核/应用检测 | `src/pages/_layout.tsx` lines 4057-4161 | **完全符合 (Fully Compliant)** | 提供包含 4 个项目的下拉列表控制更新界面。 |
| **十三 (13) / BUG-072** | 订阅卡片右键编辑上下文菜单 | `src/pages/_layout.tsx` lines 1560-1566 | **完全符合 (Fully Compliant)** | 右键提供完整的编辑、更新、定位菜单选项。 |
| **十四 (14) / BUG-065** | 活跃出口点击子集轮换限制 | `src/pages/_layout.tsx` lines 460-538 | **完全符合 (Fully Compliant)** | 出口节点点击轮换时，在 filter 与 search 约束后的活跃子集里就地递增。 |
| **十五 (15) / BUG-074** | 内核降级代理更新与版本 Ver. 前缀格式化 | `src-tauri/src/core/core_updater.rs`<br>`src/pages/_layout.tsx` lines 348-352 | **完全符合 (Fully Compliant)** | 应用了 clean 机制剥离前导 v 字符，呈现 Ver.X.Y.Z。 |
| **十六 (16) / BUG-075** | 系统资源 I/O 调优，增加延迟测速时间 | `src/hooks/use-traffic-monitor.ts` line 54 | **完全符合 (Fully Compliant)** | 默认采集间隔增宽为 3000ms，以优化空闲态资源开销。 |
| **十七 (17) / BUG-078** | 下载更新超时 20 秒控制与 fallback 机制 | `src-tauri/src/core/core_updater.rs` lines 111-120, 228-262 | **完全符合 (Fully Compliant)** | 内置了 chunk 计数超时并从本机、系统代理等多重网络链路 Fallback。 |
| **十八 (18) / BUG-079** | 倾向气泡文案统一 | `src/pages/_layout.tsx` lines 3222-3224 | **完全符合 (Fully Compliant)** | 统一了 tooltip 文字并扩展到 13 种内置 locale 配置中。 |
| **十九 (19) / BUG-080** | 相同版本重复检查提示拦截 | `src/pages/_layout.tsx` lines 1729-1733, 1779-1782 | **完全符合 (Fully Compliant)** | 检查到本地版本与远程等同时，不弹出更新面板，直接给于提示。 |
| **二十 (20) / BUG-082** | Windows 后台 WebView2 隐退内存回收机制 | `src-tauri/src/utils/window_manager.rs` lines 345-385 | **完全符合 (Fully Compliant)** | 引入 SetMemoryUsageTargetLevel 控制机制，实现前后台运行的内存降载。 |
| **二十一 (21) / BUG-089** | [已废弃] 后台探活延迟监测 | N/A | **N/A** | 该条款已弃用，功能已完全并入二十五条。 |
| **二十二 (22) / BUG-090** | Trump-3D 深色文字高对比度 | `src/pages/_layout.tsx` | **完全符合 (Fully Compliant)** | 在 Retro Dark 下使用特制文字前景色 `#2C1F03`，修复了对比度缺陷。 |
| **二十三 (23) / BUG-091** | 订阅输入框复制粘贴右键菜单 | `src/pages/_layout.tsx` lines 1452-1557 | **完全符合 (Fully Compliant)** | 精细捕获选区指针，为输入对话框附带了原生右键右键菜单。 |
| **二十四 (24) / BUG-092** | 测速后自动指向子集内最快节点 | `src/components/proxy/proxy-groups.tsx` line 408 | **完全符合 (Fully Compliant)** | 测速完毕后，自适应定位到受检索限制的活跃节点集中的最低延迟点。 |
| **二十五 (25) / BUG-083/093** | 后端常驻活跃监测守护程序，3次校验重连 | `src-tauri/src/module/monitor.rs` lines 405-494 | **完全符合 (Fully Compliant)** | 实现了后台活跃连接质量 daemon 周期维护，具有容错及自动切换算法。 |
| **二十六 (26) / PortSafety** | 启动端口冲突自动递增安全逻辑 | `src-tauri/src/config/clash.rs` lines 62-71, 472-498 | **完全符合 (Fully Compliant)** | 使用 `"adapted-by-qiu-yuxiao"` 安全校验密文，遇冲突时自动累进空闲端口。 |

*注：对于协议四（托盘图标），为规避 Windows 资源加载时的异常（BUG-073 描述的 E_FAIL 闪退），代码实现固定加载了本地 PNG 格式图层，未实现高频闪烁变换，这是为了满足高稳定性做出的工程权衡，仍判定为合理。*

---

## 🚀 总结建议 (Audit Recommendation)

为提升系统的安全性、性能与架构健壮度，建议在后续的开发迭代中，针对发现的缺陷执行以下优化动作：

1.  **并发锁重构**：按报告第一部分第3节，将 `ws_traffic` 等链式异步调用的 RwLockReadGuard 的获取包裹在局部代码块中，防止死锁。
2.  **异步化改造**：将 `std::fs` 文件 I/O 与 `sysinfo` 进程扫描全部迁移至 `tokio::fs` 或使用 `spawn_blocking` 包裹，彻底解开对 Tokio 工作线程的阻塞锁。
3.  **启动模块去阻塞**：移去 Setup 钩子中的 `block_on`，改为通过异步守护进程处理启动下载。
4.  **前端组件原子化**：全面拆分上帝布局文件 `_layout.tsx`，提取出独立的状态维护面板（Panel），将 `any` 类型声明用精确的 interface 替换。
5.  **前端缩放防噪**：使用 `useMediaQuery` 代替 `useWindowWidth` 进行极简视窗大小判定。
6.  **64位时间戳保持**：修改 `PrfItem` 对应的类型声明，使用 `i64` 时间戳格式，废除 `as usize` 截断。
