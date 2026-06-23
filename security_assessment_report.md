# Clash Mini 安全评估报告

> **评估日期**: 2026-06-23
> **评估范围**: Clash Mini v1.6.5（Tauri + React + Rust）
> **评估类型**: 全量安全审计（架构、代码、依赖、配置）

---

## 风险总览

| 严重程度 | 数量 | 需立即修复 |
|---------|:----:|:---------:|
| 🔴 严重 (Critical) | 4 | ✅ |
| 🟠 高危 (High) | 4 | ✅ |
| 🟡 中危 (Medium) | 5 | ⏰ 本周 |
| 🔵 低危 (Low) | 5 | 📋 下次迭代 |
| ℹ️ 信息 (Info) | 2 | — |

---

## 🔴 严重 (Critical) — 需立即修复

### C-1. CSP 完全禁用，无 XSS 防御

**位置**: `src-tauri/tauri.conf.json:70`
**类型**: CWE-1021 (Improper Restriction of Rendered UI Layers)
**可信度**: 高

**问题**: `"csp": null` 表示完全禁用内容安全策略。WebView 中没有任何 XSS 防护，攻击者可加载任意外部脚本、执行内联脚本、通过 `data:` URI 注入恶意代码。

**修复**: 在 `tauri.conf.json` 中设置严格的 CSP：

```json
"security": {
  "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' asset: https://asset.localhost data:; connect-src 'self' ws://localhost:* http://localhost:* https:; font-src 'self' data:; object-src 'none'; frame-src 'none'; form-action 'self'; base-uri 'none'"
}
```

**验证**: 启动应用后打开 DevTools → Console，确认无 CSP 相关错误。

---

### C-2. Shell 权限完全开放 — 前端可执行任意系统命令

**位置**: `src-tauri/capabilities/migrated.json:72-76`
**类型**: CWE-78 (OS Command Injection)
**可信度**: 高

**问题**: `migrated.json` 赋予前端 `shell:allow-execute`、`shell:allow-spawn`、`shell:allow-kill`、`shell:allow-stdin-write` 权限。任何 XSS 或恶意订阅内容都可以让攻击者在前端 JS 中通过 `@tauri-apps/plugin-shell` 执行任意系统命令（`cmd.exe /c ...` 或 `powershell ...`），完全控制用户机器。

**修复**:

```json
// src-tauri/capabilities/migrated.json
// 删除以下危险权限:
// "shell:allow-execute",
// "shell:allow-spawn",
// "shell:allow-kill",
// "shell:allow-stdin-write",
// 仅保留必要的:
"shell:allow-open"
```

然后在 Rust 后端对 `open_web_url` 做 URL 验证（见 H-1 修复）。

**验证**: 确认前端代码中没有使用 `Command.create()`、`Command.execute()` 等 shell 执行调用。

---

### C-3. FS 作用域设为 `**` — 前端可读写任意文件

**位置**: `src-tauri/capabilities/migrated.json:11-13`
**类型**: CWE-22 (Path Traversal)
**可信度**: 高

**问题**: `"fs:scope": { "allow": ["$APPDATA/**", "$RESOURCE/../**", "**"] }` 中的 `"**"` 意味着前端可以通过 `@tauri-apps/plugin-fs` 读取和写入文件系统上**任何**文件，包括系统文件（`C:\Windows\*`）、用户文档、浏览器数据等。

**修复**:

```json
{
  "identifier": "fs:scope",
  "allow": [
    "$APPDATA/**",
    "$RESOURCE/**"
  ]
}
```

移除 `"$RESOURCE/../**"`（逃逸到资源目录上级）和 `"**"`（通配所有路径）。

**验证**: 在 DevTools 中执行 `import { readTextFile } from '@tauri-apps/plugin-fs'; await readTextFile('C:\\Windows\\win.ini')` 应被拒绝。

---

### C-4. Asset Protocol 允许读取任意本地文件

**位置**: `src-tauri/tauri.conf.json:61-68`
**类型**: CWE-200 (Information Exposure)
**可信度**: 高

**问题**: `assetProtocol.scope.allow: ["**"]` 允许通过 `asset://localhost/<path>` 协议读取任意本地文件。结合 `convertFileSrc()` 的使用，任何下载到本地的文件都可以被 WebView 加载，包括恶意 HTML/JS。

**修复**:

```json
"assetProtocol": {
  "enable": true,
  "scope": {
    "allow": [
      "$APPDATA/**",
      "$RESOURCE/**"
    ],
    "requireLiteralLeadingDot": false
  }
}
```

**验证**: 尝试在浏览器中访问 `asset://localhost/C:/Windows/win.ini` 应被拒绝。

---

## 🟠 高危 (High) — 需立即修复

### H-1. `open_web_url` 命令无 URL 协议验证

**位置**: `src-tauri/src/cmd/app.rs:31`
**类型**: CWE-939 (Improper Authorization in Handler for Custom URL Scheme)
**可信度**: 高

**问题**: `open_web_url(url: String)` 将前端传入的任意 URL 直接传递给 `open::that()`。攻击者可通过 XSS 或恶意订阅注入构造 `file:///C:/Windows/system32/cmd.exe` 或 `javascript:...` 等恶意链接。

**修复**:

```rust
// src-tauri/src/cmd/app.rs
use url::Url;

#[tauri::command]
pub fn open_web_url(url: String) -> CmdResult<()> {
    let parsed = Url::parse(&url).map_err(|_| "invalid URL")?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err("only http/https URLs are allowed".into());
    }
    open::that(url.as_str()).stringify_err()
}
```

**验证**: 传入 `file:///C:/Windows/win.ini` 应返回错误。

---

### H-2. ZIP 解压无路径遍历防护 (Zip Slip)

**位置**: `src-tauri/src/feat/backup.rs:133,316`
**类型**: CWE-22 (Path Traversal: Zip Slip)
**可信度**: 高

**问题**: 备份恢复时 `zip.extract(app_home_dir()?)` 直接解压 ZIP 文件，没有对 ZIP 条目中的路径进行 `../` 检查。恶意构造的备份文件可覆写应用目录外的任意文件，例如覆写 `verge.yaml`、注入恶意 JS 脚本覆盖 Script 文件、或覆写 sidecar 二进制实现 RCE。

**修复**:

```rust
// src-tauri/src/feat/backup.rs
use std::path::Path;

fn safe_extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_path = entry.mangled_name();
        // Zip Slip 防护：拒绝包含 ".." 或为绝对路径的条目
        let entry_path = entry_path
            .components()
            .try_fold(dest.to_path_buf(), |base, comp| {
                match comp {
                    std::path::Component::ParentDir => {
                        // 拒绝 ".." 组件
                        Err(anyhow::anyhow!("ZIP entry contains '..' path component: {:?}", entry_path))
                    }
                    std::path::Component::RootDir => {
                        // 拒绝绝对路径条目
                        Err(anyhow::anyhow!("ZIP entry contains absolute path: {:?}", entry_path))
                    }
                    _ => Ok(base.join(comp)),
                }
            })?;

        // 确保在目标目录内
        if !entry_path.starts_with(dest) {
            bail!("ZIP entry escapes target directory: {:?}", entry_path);
        }

        if entry.is_dir() {
            fs::create_dir_all(&entry_path)?;
        } else {
            if let Some(parent) = entry_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&entry_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    Ok(())
}
```

然后在 `restore_webdav_backup` 和 `restore_local_backup` 中替换为 `safe_extract_zip`。

**验证**: 构造一个包含 `../../danger.txt` 条目的 ZIP 文件，确认解压被拒绝。

---

### H-3. 订阅 URL 的 `accept_invalid_certs` 上游可控 — MITM 风险

**位置**: `src-tauri/src/config/prfitem.rs:439`
**类型**: CWE-295 (Improper Certificate Validation)
**可信度**: 中

**问题**: `PrfOption` 中的 `danger_accept_invalid_certs` 字段可由用户配置，当设置为 `true` 时，订阅下载将跳过 TLS 证书验证，使 MITM 攻击成为可能。恶意订阅服务器或网络中间人可以发送篡改的订阅内容，注入恶意节点配置。

**修复**: 在 `PrfItem::from_url` 中添加日志警告，并在 UI 层面明确标记此选项为危险功能：

```rust
// src-tauri/src/config/prfitem.rs:439
let accept_invalid_certs = option.is_some_and(|o| o.danger_accept_invalid_certs.unwrap_or(false));
if accept_invalid_certs {
    logging!(
        warn,
        Type::Config,
        "⚠️ 订阅使用了危险选项 `danger_accept_invalid_certs=true`，TLS 证书验证被跳过！"
    );
}
```

**验证**: 在 UI 上检查「跳过证书验证」选项是否有安全警告提示。

---

### H-4. 嵌入的 JavaScript 引擎 (boa_engine) 执行用户脚本

**位置**: `src-tauri/src/enhance/script.rs`
**类型**: CWE-94 (Code Injection)
**可信度**: 中

**问题**: 应用使用 `boa_engine` 执行用户编写的 JavaScript 脚本来配置增强。虽然脚本是从本地文件读取（非远程下载），但脚本内容通过配置文件编辑写入。如果攻击者有能力写入配置文件，可以直接注入恶意 JS 代码。此外，`name` 参数被注入到 JS 代码字符串中（第 102 行），虽然做了转义，但这种方式本身存在风险。

现有防护（较好）：循环限制、超时、输出大小限制、JSON 大小限制已实现。

**建议**: 保持现有运行时限制的基础上，增加：
1. 禁止 `require`、`import`、`fetch`、`XMLHttpRequest` 等 API
2. 移除 `constructor`、`__proto__` 等原型链访问
3. 使用 `Context::builder()` 限制可用全局对象

```rust
// 在 use_script_sync 中增加
let mut context = Context::default();
// 移除原型链操作能力
context.eval(Source::from_bytes(
    r#"Object.freeze(Object.prototype);
    Object.freeze(Function.prototype);
    delete globalThis.constructor;
    "#,
)).ok();
```

**验证**: 尝试在脚本中执行 `this.constructor.constructor('return process')()` 应被禁止。

---

## 🟡 中危 (Medium) — 本周修复

### M-1. `download_icon_cache` 下载任意 URL 到本地

**位置**: `src-tauri/src/cmd/app.rs:99-103`
**类型**: CWE-918 (Server-Side Request Forgery)
**可信度**: 高

**问题**: `download_icon_cache(url, name)` 接受任意 URL 并下载到本地 `icons/cache/` 目录。虽然文件名做了路径遍历防护，但 URL 协议未做验证。攻击者可利用 `file:///`、`ftp://` 等协议，或 SSRF 访问内网资源。

**修复**:

```rust
// src-tauri/src/feat/icon.rs:73
pub async fn download_icon_cache(url: String, name: String) -> CmdResult<String> {
    // 验证 URL 协议
    let parsed = url::Url::parse(&url).map_err(|_| "invalid URL")?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("only http/https URLs are allowed for icon download".into());
    }

    // 限制 IP（防止 SSRF 到内网）
    if let Some(host) = parsed.host() {
        match host {
            url::Host::Domain(d) => {
                // 拒绝 localhost
                if d == "localhost" || d == "127.0.0.1" || d == "::1" {
                    return Err("cannot download from localhost".into());
                }
            }
            url::Host::Ipv4(ip) => {
                if ip.is_loopback() || ip.is_private() || ip.is_unspecified() {
                    return Err("cannot download from private/loopback IP".into());
                }
            }
            url::Host::Ipv6(ip) => {
                if ip.is_loopback() || ip.is_unspecified() {
                    return Err("cannot download from loopback IP".into());
                }
            }
        }
    }

    // ... 后续下载逻辑
}
```

**验证**: 尝试传入 `file:///C:/Windows/win.ini` 应被拒绝。

---

### M-2. 订阅导入 URL 无 SSRF 防护

**位置**: `src-tauri/src/config/prfitem.rs:257-643`
**类型**: CWE-918 (SSRF)
**可信度**: 中

**问题**: `PrfItem::from_url` 对用户提供的订阅 URL 做了基本校验（`fix_dirty_url` 函数），但没有 IP 黑名单 / 内网地址防护。用户可构造 `http://192.168.1.1/` 或 `http://[::1]:8080/` 等 URL 来探测内网服务。

**修复**: 在 `fix_dirty_url` 或 `from_url` 入口增加 SSRF 防护：

```rust
// src-tauri/src/config/prfitem.rs
fn validate_url_no_ssrf(url: &Url) -> Result<()> {
    if let Some(host) = url.host() {
        match host {
            url::Host::Domain(d) => {
                if d == "localhost" || d == "127.0.0.1" || d == "::1" || d == "0.0.0.0" {
                    bail!("cannot fetch subscription from localhost");
                }
            }
            url::Host::Ipv4(ip) => {
                if ip.is_loopback() || ip.is_private() || ip.is_unspecified() {
                    bail!("cannot fetch subscription from private/loopback IP");
                }
            }
            url::Host::Ipv6(ip) => {
                if ip.is_loopback() || ip.is_unspecified() {
                    bail!("cannot fetch subscription from loopback IP");
                }
            }
        }
    }
    Ok(())
}
```

**验证**: 尝试导入 `http://127.0.0.1:7777/config` 应被拒绝。

---

### M-3. HTTP 插件允许访问任意 URL（前端 SSRF）

**位置**: `src-tauri/capabilities/desktop.json:25-27`
**类型**: CWE-918 (SSRF)
**可信度**: 中

**问题**: `"http:default"` 的 `allow` 规则为 `[{ "url": "https://*/*" }, { "url": "http://*/*" }]`，前端可通过 `@tauri-apps/plugin-http` 访问**任何** HTTP/HTTPS URL，包括内网服务。

**修复**:

```json
{
  "identifier": "http:default",
  "allow": [
    { "url": "https://*cdn.jsdelivr.net/*" },
    { "url": "https://*githubusercontent.com/*" },
    { "url": "https://*github.com/*" },
    { "url": "https://*raw.githubusercontent.com/*" }
  ]
}
```

如果业务需要用户配置任意 URL（如自定义订阅源），应通过 Rust 后端转发而非前端直连。

**验证**: 确认前端代码中没有使用 `@tauri-apps/plugin-http` 直接发起任意网络请求。

---

### M-4. 深链接协议无参数验证

**位置**: `src-tauri/tauri.conf.json:46-53`
**类型**: CWE-939 (Improper Authorization in Handler for Custom URL Scheme)
**可信度**: 低

**问题**: 应用注册了 `clash://` 和 `clash-mini://` 协议处理。如果在 Rust 后端处理深链接 URL 时未做验证，攻击者可通过网页链接或 HTML 邮件触发应用执行未授权操作。

**修复**: 确认深链接处理函数中对 URL 参数做了严格校验，仅允许白名单内的操作：

```rust
// 深链接处理示例
fn handle_deep_link(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url)?;
    match parsed.host_str() {
        Some("import") => {
            let url_param = parsed.query_pairs()
                .find(|(k, _)| k == "url")
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| anyhow!("missing url parameter"))?;
            // 对 url_param 做额外 SSRF 防护...
            import_profile(url_param).await?;
        }
        Some("open") => {
            // 严格限制允许的操作
        }
        _ => bail!("unknown deep link action"),
    }
    Ok(())
}
```

**验证**: 在浏览器中点击 `clash://dangerous-action?payload=xxx` 应被拒绝。

---

### M-5. `use_script` 的 JS 代码拼接存在注入风险

**位置**: `src-tauri/src/enhance/script.rs:102-109`
**类型**: CWE-94 (Code Injection)
**可信度**: 中

**问题**: `name` 和 `config_str` 通过字符串拼接注入到 JS 代码模板中。虽然 `name` 做了 `escape_js_string_for_single_quote` 转义（反斜杠和单引号），但转义不够完善，没有处理 `${}` 模板字面量注入。

**修复**: 使用 JSON 传递 name 参数以彻底避免注入：

```rust
let safe_name_json = serde_json::to_string(&name)?;
let safe_config_json = serde_json::to_string(&config)?;

let code = format!(
    r"try{{
        const __verge_config = JSON.parse({safe_config_json});
        const __verge_name = JSON.parse({safe_name_json});
        {script};
        var __result = main(__verge_config, __verge_name);
        JSON.stringify(__result || '');
    }} catch(err) {{
        `__error_flag__ ${{err.toString()}}`
    }}"
);
```

**验证**: 构造包含 `'`、`\`、`${}` 等特殊字符的 name，确认脚本执行不报异常。

---

## 🔵 低危 (Low) — 下次迭代

### L-1. WebDAV 密码明文存储

**位置**: `src-tauri/src/types/verge.ts:124-128`
**类型**: CWE-312 (Cleartext Storage of Sensitive Information)

**问题**: WebDAV 密码以明文存储在 `verge.yaml` 配置文件中。任何能读取该文件的进程或用户都可获取密码。

**建议**: 使用 Tauri 的 `safe_storage` 插件或操作系统的密钥链（Keychain / Credential Manager）存储敏感凭证。

---

### L-2. 配置文件无完整性校验

**位置**: 全局
**类型**: CWE-354 (Improper Validation of Integrity Check Value)

**问题**: 订阅内容从远程下载后直接保存为 YAML 文件，没有签名或完整性校验。如果订阅源被中间人攻击（且未启用有效的 TLS 验证），可注入恶意配置。

**建议**: 对订阅内容增加 SHA-256 校验或 GPG 签名验证，让用户可与订阅提供方约定校验方式。

---

### L-3. `localStorage` 未加密存储敏感状态

**位置**: 多处（`src/pages/unlock.tsx`、`src/components/proxy/use-head-state.ts` 等）
**类型**: CWE-312 (Cleartext Storage)

**问题**: 流媒体解锁结果、代理链配置等存储在无加密的 `localStorage` 中。在 Tauri WebView 中，localStorage 存储在本地 SQLite 文件中，可被其他进程读取。

**建议**: 敏感数据应通过 Rust 后端加密后存储。非敏感数据需注意避免存储用户可识别的代理信息。

---

### L-4. YAML `serde_yaml_ng::from_str` 无上限限制

**位置**: 多处
**类型**: CWE-400 (Uncontrolled Resource Consumption)

**问题**: 解析远程订阅的 YAML 内容时未限制反序列化大小，构造超大的嵌套 YAML 可导致内存耗尽。

**建议**: 在 YAML 解析前检查内容大小（例如限制 < 50MB），或使用 `serde_yaml_ng` 的 `Deserializer::from_str` 手动限制递归深度。

---

### L-5. Release 配置关闭了 overflow-checks

**位置**: `Cargo.toml:20`
**类型**: CWE-190 (Integer Overflow)

**问题**: `profile.release` 中 `overflow-checks = false`，Rust 的整数溢出在 release 模式下会静默回绕（wrapping），可能导致意外的逻辑错误或安全漏洞。

**建议**: 将 `overflow-checks` 设置为 `true` 以在 debug 和 release 模式下都捕获整数溢出：

```toml
[profile.release]
overflow-checks = true
```

---

## ℹ️ 信息 (Info)

### I-1. Clash API Secret 暴露到前端

**位置**: `src/services/cmds.ts:144`
**类型**: 架构信息

Clash 核心的 API secret 通过 IPC 传递到前端（`getClashInfo`），这是 Tauri 的标准模式 — secret 仅在 IPC 通道中传递，不会暴露给外部网络。但需要注意：前端代码不应在日志或 UI 中明文显示此 secret。

### I-2. `removeUnusedCommands: true` 已配置

**位置**: `src-tauri/tauri.conf.json:32`

这是一个好的安全配置，Tauri 会在构建时自动移除未使用的命令，减少攻击面。

---

## 加固优先级行动计划

### 🔴 立即修复（今天）

| 优先级 | 问题 | 预估工时 |
|:------:|------|:-------:|
| 1 | C-2: 移除 shell 危险权限 | 10min |
| 2 | C-1: 设置 CSP | 10min |
| 3 | C-3: 限制 FS 作用域 | 10min |
| 4 | C-4: 限制 asset protocol 范围 | 10min |
| 5 | H-1: URL 协议验证 | 15min |
| 6 | H-2: ZIP 解压防护 | 30min |

### 🟡 本周修复

| 优先级 | 问题 | 预估工时 |
|:------:|------|:-------:|
| 7 | M-1: 图标下载 URL 验证 | 20min |
| 8 | M-2: 订阅 URL SSRF 防护 | 30min |
| 9 | M-5: JS 代码注入加固 | 15min |
| 10 | H-3: 日志警告危险选项 | 10min |

### 🔵 下次迭代

| 优先级 | 问题 | 预估工时 |
|:------:|------|:-------:|
| 11 | M-3: HTTP 插件作用域限制 | 15min |
| 12 | M-4: 深链接参数验证 | 20min |
| 13 | L-1~L-5: 低风险项 | 各约 15-30min |

---

## 知识库参考

> ⚠️ 内网知识库未覆盖此项目的具体漏洞场景 — 以上为通用安全加固建议。
> 建议与安全团队确认是否存在针对 Tauri 应用的企业级安全基线和扫描工具。

---

## 变更记录

| 日期 | 版本 | 审核人 | 变更说明 |
|------|------|--------|---------|
| 2026-06-23 | v1.0 | — | 初始安全评估报告 |
