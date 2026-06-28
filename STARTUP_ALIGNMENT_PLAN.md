# Clash Mini 启动流程对齐 Clash Verge Rev 计划

**制定时间**: 2026-06-28  
**目标**: 将 Clash Mini 的启动流程对齐 Clash Verge Rev，解决内核启动不稳定的根本问题

---

## 📋 问题根因

Clash Mini 的 `CoreManager` 缺少启动保护机制：

| 缺失机制 | 后果 |
|---------|------|
| 无 `lifecycle_lock` | 竞态条件：多个任务同时启动/停止内核 |
| 无幂等性检查 | 内核已运行时重复启动，状态混乱 |
| 无退出检查 | 程序退出期间仍尝试启动内核 |
| 无失败回滚 | 启动失败后状态卡死，后续无法启动 |
| 无 `config_update_in_progress` | 配置更新与内核重启交叉，未定义行为 |

---

## ✅ 修改计划（分6步，每步可独立验证）

### 第1步：修改 `core/manager/mod.rs` - 添加缺失字段

**文件**: `src-tauri/src/core/manager/mod.rs`

**添加字段**：
```rust
pub struct CoreManager {
    state: ArcSwap<State>,
    last_update: ArcSwapOption<Instant>,
    // 新增 ↓
    config_update_in_progress: AtomicBool,
    lifecycle_lock: tokio::sync::Mutex<()>,
    #[cfg(target_os = "windows")]
    handoff_watcher_running: AtomicBool,
}
```

**修改 Default 实现**：
```rust
impl Default for CoreManager {
    fn default() -> Self {
        Self {
            state: ArcSwap::new(Arc::new(State::default())),
            last_update: ArcSwapOption::new(None),
            // 新增 ↓
            config_update_in_progress: AtomicBool::new(false),
            lifecycle_lock: tokio::sync::Mutex::new(()),
            #[cfg(target_os = "windows")]
            handoff_watcher_running: AtomicBool::new(false),
        }
    }
}
```

**添加辅助方法**：
```rust
impl CoreManager {
    // 新增 ↓
    fn try_start_config_update(&self) -> bool {
        !self.config_update_in_progress.swap(true, Ordering::AcqRel)
    }

    fn finish_config_update(&self) {
        self.config_update_in_progress.store(false, Ordering::Release);
    }
}
```

**验证方法**：`cargo check` 编译通过

---

### 第2步：修改 `core/manager/lifecycle.rs` - 重写 `start_core()` 和 `stop_core()`

**文件**: `src-tauri/src/core/manager/lifecycle.rs`

**2.1 重写 `start_core()`**：
```rust
pub async fn start_core(&self) -> Result<()> {
    let _life = self.lifecycle_lock.lock().await;
    self.start_core_inner().await
}
```

**2.2 添加 `start_core_inner()`**（新的内部函数）：
```rust
/// 调用者须已持有 `lifecycle_lock`。
async fn start_core_inner(&self) -> Result<()> {
    // 退出中不再启动新内核
    if Handle::global().is_exiting() {
        return Ok(());
    }

    // 已有内核运行时保持幂等
    if !matches!(*self.get_running_mode(), RunningMode::NotRunning) {
        logging!(info, Type::Core, "start_core called while a core is running; treated as no-op");
        return Ok(());
    }

    self.prepare_startup().await;
    defer! { self.after_core_process(); }

    // 等待服务期间可能进入退出
    if Handle::global().is_exiting() {
        self.set_running_mode(RunningMode::NotRunning);
        return Ok(());
    }

    let result = match *self.get_running_mode() {
        RunningMode::Service => self.start_core_by_service().await,
        RunningMode::NotRunning | RunningMode::Sidecar => self.start_core_by_sidecar().await,
    };

    // 启动失败时回滚 mode
    if result.is_err() {
        self.set_running_mode(RunningMode::NotRunning);
    }

    #[cfg(target_os = "windows")]
    if result.is_ok() && matches!(*self.get_running_mode(), RunningMode::Sidecar) {
        self.spawn_service_handoff_watcher().await;
    }

    result
}
```

**2.3 重写 `stop_core()`**：
```rust
pub async fn stop_core(&self) -> Result<()> {
    let _life = self.lifecycle_lock.lock().await;
    self.stop_core_inner().await
}
```

**2.4 添加 `stop_core_inner()`**（新的内部函数）：
```rust
/// 调用者须已持有 `lifecycle_lock`。
async fn stop_core_inner(&self) -> Result<()> {
    CLASH_LOGGER.clear_logs().await;
    defer! { self.after_core_process(); }

    match *self.get_running_mode() {
        RunningMode::Service => self.stop_core_by_service().await,
        RunningMode::Sidecar => {
            self.stop_core_by_sidecar();
            Ok(())
        }
        RunningMode::NotRunning => Ok(()),
    }
}
```

**2.5 重写 `restart_core()`**：
```rust
pub async fn restart_core(&self) -> Result<()> {
    let _life = self.lifecycle_lock.lock().await;
    logging!(info, Type::Core, "Restarting core");
    self.stop_core_inner().await?;
    self.start_core_inner().await
}
```

**验证方法**：
1. `cargo check` 编译通过
2. 启动程序，观察是否立即出现 `mini-mihomo.exe` 进程
3. 查看日志，确认 `start_core_inner` 被调用

---

### 第3步：添加 Windows service handoff 逻辑（可选，TUN 模式相关）

**文件**: `src-tauri/src/core/manager/lifecycle.rs`

**3.1 添加 `spawn_service_handoff_watcher()`**：
```rust
#[cfg(target_os = "windows")]
async fn spawn_service_handoff_watcher(&self) {
    // ... (从 Clash Verge Rev 复制)
}
```

**3.2 添加 `try_handoff_sidecar_to_service()`**：
```rust
#[cfg(target_os = "windows")]
async fn try_handoff_sidecar_to_service(&self) -> HandoffOutcome {
    // ... (从 Clash Verge Rev 复制)
}
```

**3.3 添加 `HandoffOutcome` 枚举**：
```rust
#[cfg(target_os = "windows")]
enum HandoffOutcome {
    NotReady,
    Done,
    Failed,
}
```

**验证方法**：
1. 启用 TUN 模式
2. 启动程序
3. 观察是否从 sidecar 模式自动切换到 service 模式

---

### 第4步：对齐 `resolve/mod.rs` 的启动顺序

**文件**: `src-tauri/src/utils/resolve/mod.rs`

**当前顺序**（Clash Mini）：
```rust
init_verge_config().await;      // 第63行
Config::verify_config_initialization().await;  // 第64行
init_window().await;            // 第65行
// 然后异步启动内核
```

**Clash Verge Rev 顺序**（第51-86行）：
```rust
init_verge_config().await;      // 第63行
Config::verify_config_initialization().await;  // 第64行
init_window().await;            // 第65行
// 然后异步启动内核（相同）
```

**结论**: 启动顺序已经一致，无需修改。

**但需添加日志**（便于调试）：
```rust
pub fn resolve_setup_async() {
    AsyncHandler::spawn(move || async move {
        logging!(info, Type::Setup, "Starting async setup...");
        
        init_verge_config().await;
        logging!(info, Type::Setup, "Verge config initialized");
        
        Config::verify_config_initialization().await;
        logging!(info, Type::Setup, "Config verification completed");
        
        init_window().await;
        logging!(info, Type::Setup, "Window created");
        
        // ...
    });
}
```

**验证方法**：查看日志，确认每个步骤都成功执行

---

### 第5步：添加详细的错误日志

**文件**: `src-tauri/src/core/manager/lifecycle.rs`

在关键位置添加日志：
```rust
async fn start_core_inner(&self) -> Result<()> {
    logging!(info, Type::Core, "start_core_inner called");
    
    if Handle::global().is_exiting() {
        logging!(info, Type::Core, "Exiting, skipping core start");
        return Ok(());
    }
    
    // ...
    
    let result = match *self.get_running_mode() {
        // ...
    };
    
    if let Err(ref e) = result {
        logging!(error, Type::Core, "Core start failed: {}", e);
    }
    
    result
}
```

**验证方法**：如果启动失败，日志会显示具体原因

---

### 第6步：测试与验证

**测试矩阵**：

| 测试场景 | 预期结果 |
|---------|---------|
| 正常启动 | 立即出现 `mini-mihomo.exe` 进程 |
| 静默启动 | 后台出现 `mini-mihomo.exe` 进程 |
| 快速重启 | 无竞态，稳定重启 |
| 退出时启动 | 不启动内核，直接退出 |
| TUN 模式启动 | 自动从 sidecar 切换到 service |
| 导入订阅后 | 内核正常运行 |

**验证方法**：
1. 编译：`pnpm build && cargo build --release`
2. 运行：`target/release/clash-mini.exe`
3. 观察：任务管理器是否立即出现 `mini-mihomo.exe`
4. 日志：查看 `%APPDATA%\clash-mini\logs\latest.log`

---

## 📊 风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 引入新 bug | 低 | 高 | 分步验证，每步可回滚 |
| 编译失败 | 低 | 低 | `cargo check` 提前验证 |
| 功能回归 | 低 | 中 | 保留轻量模式等特有功能 |
| 性能下降 | 极低 | 低 | `lifecycle_lock` 仅在启动/停止时持有 |

---

## 🚀 实施顺序

1. **第1步**（必须）：修改 `mod.rs`，添加字段
2. **第2步**（必须）：修改 `lifecycle.rs`，添加锁保护
3. **第3步**（可选）：添加 service handoff（TUN 模式）
4. **第4步**（推荐）：添加启动日志
5. **第5步**（推荐）：添加错误日志
6. **第6步**（必须）：测试验证

---

## 📝 后续优化（不在本计划内）

1. 对齐 Clash Verge Rev 的 `verify_config_initialization()` 重试逻辑
2. 对齐配置文件生成逻辑
3. 添加启动超时保护
4. 优化轻量模式与启动流程的交互

---

## ✅ 完成标准

- [ ] 第1步完成：`mod.rs` 编译通过
- [ ] 第2步完成：`lifecycle.rs` 编译通过，内核启动稳定
- [ ] 第3步完成（可选）：TUN 模式 handoff 正常
- [ ] 第4步完成（可选）：启动日志完整
- [ ] 第5步完成（可选）：错误日志完整
- [ ] 第6步完成：所有测试场景通过

---

**等待批准**：是否按照此计划执行？还是有需要修改的地方？
