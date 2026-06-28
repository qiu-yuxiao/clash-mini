# Clash Mini IPC & API 安全审查报告

**审查日期**：2026-06-28  
**审查人**：接口探（API测试专家）  
**范围**：Tauri IPC 命令层（69 个 command）+ Mihomo 核心 API 通信

---

## 一、整体评估

**结论：安全，无严重漏洞。** IPC 命令层的设计整体稳健，输入来源受控，敏感操作有锁保护。发现的均属防御性加固建议。

---

## 二、发现项

### 🟡 S1：`save_webdav_config` 密码明文存储

**文件**：`src-tauri/src/cmd/webdav.rs:12`  
**问题**：WebDAV 密码通过 `SmartString` 明文写入 `verge.yaml`：
```rust
pub async fn save_webdav_config(url: String, username: String, password: String) -> CmdResult<()> {
    let patch = IVerge {
        webdav_password: Some(password),  // 明文
        ...
    };
```
**影响**：配置文件在磁盘上明文包含远程存储凭据。Windows 下应用目录 `%APPDATA%` 虽为私有文件夹，但恶意程序可能读取。  
**缓解已有**：`get_export_conf`（`backup.rs:275-277`）导出配置时会扒除凭据——好的。  
**建议**：考虑用 OS 凭据管理器（Windows Credential Manager / macOS Keychain）替代明文存储。

---

### 🟡 S2：`delete_webdav_backup` / `restore_webdav_backup` 文件名无校验

**文件**：`src-tauri/src/cmd/webdav.rs:42,48`  
**问题**：`filename: String` 参数直接拼接到远程路径，无长度限制、无非法字符过滤。  
**影响**：若前端被 XSS/注入攻破，恶意文件名可能触发服务端路径遍历。当前前端不会传非法值，属于纵深防御缺失。  
**建议**：校验 `filename` 仅包含 `[a-zA-Z0-9._-]`，长度 ≤ 255。

---

### 🟡 S3：`get_proxy_addr` 读取 `provider["path"]` 无路径校验

**文件**：`src-tauri/src/cmd/proxy.rs:29-34`  
**问题**：从 Mihomo 配置中读取 `provider["path"]`，若 path 为绝对路径或包含 `..`，可能越界读文件：
```rust
let mut provider_path = PathBuf::from(path.as_str());
if provider_path.is_relative() {
    provider_path = app_dir.join(provider_path);
}
```
**影响**：path 来自 Mihomo 内核自身配置（非用户直接输入），且只读 `server`/`port` 元数据不写入。**风险极低。**  
**建议**：加 `provider_path.starts_with(&app_dir)` 守卫。

---

### 💭 N1：WebDAV URL 协议未强校验

**文件**：`src-tauri/src/cmd/webdav.rs:12`  
**问题**：`url: String` 未检查是否为 `https://` 开头。HTTP WebDAV 密码将明文传输。  
**建议**：前端加 regex 校验，或后端 `ensure!(url.starts_with("https://"))`。

---

### 💭 N2：69 个 Tauri command 全部通过 `CmdResult<T>` 统一错误包装

所有命令使用 `Result<T, String>` → `CmdResult<T>` 的错误链，错误返回给前端显示。  
**评估**：错误信息可能泄露文件路径等内部细节（如 `save_profile.rs` 中的 `expect("failed to get profiles dir")` 会 panic 导致进程崩溃）。建议将 `expect()` 替换为 `Result` 传播。

---

### ✅ N3：已确认安全的设计

- Profile 文件写入走 `profiles.latest_arc().get_item(&index)` 查询——**无任意路径写入**
- 导出配置函数 `get_export_conf` 主动移除 `webdav_username/password/url`
- 备份模块有数量上限（`AUTO_BACKUP_KEEP = 20`）
- WebSocket 连接管理使用引用计数（`SharedWebSocketRefs`）
- 轻量模式状态机使用原子 CAS 防止竞态

---

## 三、总结

| 级别 | 数量 | 风险 |
|------|------|------|
| 🔴 Blocker | 0 | — |
| 🟡 Suggestion | 3 | 低-中 |
| 💭 Nit | 3 | 低 |

**可发布。** S1-S3 属于防御性加固，不阻塞发版。
