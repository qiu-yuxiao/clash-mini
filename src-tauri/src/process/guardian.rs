use std::{
    fs::{self, OpenOptions},
    io::{self, Read as _, Write as _},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering as AtomicOrdering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest as _, Sha256};

/// 同一开机会话 / 会话纪元内连续异常终止的最大自动拉起代数上限（A8：严禁靠时间窗清零）。
pub const MAX_RAPID_RESPAWN_GEN: u32 = 3;

/// 带外主动退出 `prepared` 阶段凭证有效期（毫秒，30s），超时视作残留旧凭证并拒绝抑制复活（C1）。
pub const INTENTIONAL_EXIT_PROOF_TTL_MS: u64 = 30_000;

/// 带外主动退出 `confirmed` 阶段凭证有效期（毫秒，120s），超时同样失效防跨重启 PID 复用误判（C1）。
pub const INTENTIONAL_EXIT_CONFIRMED_TTL_MS: u64 = 120_000;

/// 物理隔离目录中的伴生看门狗主可执行文件名（与安装目录 `clash-mini.exe` 异名异目录，防应用组连坐强杀）。
pub const ISOLATED_GUARDIAN_BINARY_NAME: &str = "clash-mini-guard.exe";

/// 当主隔离文件被短暂锁定时使用的备用隔离可执行文件名。
pub const ISOLATED_GUARDIAN_FALLBACK_NAME: &str = "clash-mini-guard-alt.exe";

/// 独立看门狗取证账本文件名（A1：脱离 `latest.log`，避免被下次启动日志轮转归档覆盖）。
pub const GUARDIAN_LOG_FILENAME: &str = "guardian.log";

/// 宿主独立诊断心跳文件名（A2：仅作排障证据，不参与复活门控）。
pub const MAIN_HEARTBEAT_FILENAME: &str = "main_heartbeat.json";

/// 看门狗独立诊断心跳文件名（A2：仅作排障证据，不参与复活门控）。
pub const GUARD_HEARTBEAT_FILENAME: &str = "guard_heartbeat.json";

/// 带外主动退出持久化凭证文件名（A7：与命名内核事件组成双真源）。
pub const INTENTIONAL_EXIT_FILENAME: &str = "intentional_exit.json";

/// 物理会话与累计复活预算状态文件名（A8）。
pub const RESPAWN_STATE_FILENAME: &str = "respawn_state.json";

/// 进程独立拉起代理名称：Session-0 WMI `Win32_Process::Create`（`WmiPrvSE.exe`）。
pub const BROKER_WMI_SESSION0: &str = "wmi_session0";

/// 进程独立拉起代理名称：Task Scheduler 2.0 COM (`ITaskService`)。
pub const BROKER_TASK_SCHEDULER_COM: &str = "task_scheduler_com";

/// 进程独立拉起代理名称：直接子进程 `CREATE_BREAKAWAY_FROM_JOB`。
pub const BROKER_BREAKAWAY_JOB: &str = "breakaway_from_job";

/// 进程独立拉起代理名称：同 Job 降级拉起。
pub const BROKER_SAME_JOB_DEGRADED: &str = "same_job_degraded";

/// 检测到主进程非预期终止后，拉起前的安全避让等待毫秒数（避让安装/卸载器与主动退出写盘竞态，A7）。
const RESPAWN_SAFETY_DELAY_MS: u64 = 3500;

/// 宿主与看门狗独立诊断心跳刷新周期（毫秒，15s）。
const HEARTBEAT_INTERVAL_MS: u64 = 15_000;

/// 伴生看门狗稳定存活超过该秒数后被杀，视为独立误杀事件，快速重建退避计数归零。
const GUARD_RAPID_DEATH_WINDOW_SECS: u64 = 60;

/// 伴生看门狗首次或稳态后意外终止的基础重建延迟毫秒数。
const GUARD_REBUILD_BASE_DELAY_MS: u64 = 1500;

/// Supervisor 遭遇安装器/卸载器运行时的最大轮询等待秒数，超时后再退出由后台巡检重武装。
const INSTALLER_WAIT_MAX_SECS: u64 = 120;

/// 进程内单调临时文件序列号，消除并发写原子 JSON 临时文件名冲突（C5）。
static ATOMIC_TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardianArgs {
    pub parent_pid: u32,
    pub target_exe: PathBuf,
    pub log_dir: PathBuf,
    pub guard_dir: Option<PathBuf>,
    pub singleton_port: u16,
    pub generation: u32,
    pub session_id: Option<String>,
    pub session_epoch: Option<String>,
    pub broker: Option<String>,
}

impl GuardianArgs {
    /// 解析看门狗状态落盘目录（优先显式 `--guard-dir`，否则使用 `%LOCALAPPDATA%\<APP_ID>\guard`）。
    pub fn resolved_guard_dir(&self) -> PathBuf {
        self.guard_dir.clone().unwrap_or_else(resolve_isolated_guardian_dir)
    }

    /// 解析当前守护链路会话标识（优先 `--session-id`，缺省按父进程 PID 合成）。
    pub fn resolved_session_id(&self) -> String {
        self.session_id
            .clone()
            .unwrap_or_else(|| format!("pid-{}", self.parent_pid))
    }

    /// 解析累计自愈预算所属会话纪元标识（优先 `--session-epoch`，退避至 `resolved_session_id()`）。
    pub fn resolved_session_epoch(&self) -> String {
        self.session_epoch.clone().unwrap_or_else(|| self.resolved_session_id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnInfo {
    pub generation: u32,
    pub prev_pid: Option<u32>,
    pub prev_exit_code: Option<u32>,
    pub silent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MainHeartbeat {
    pub pid: u32,
    pub session_id: String,
    pub in_lightweight: bool,
    pub is_exiting: bool,
    pub parent_in_job: Option<bool>,
    pub breakaway_degraded: bool,
    #[serde(default)]
    pub spawn_broker: Option<String>,
    pub guard_pid: Option<u32>,
    pub last_seen_unix_ms: u64,
    pub last_seen_local: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuardHeartbeat {
    pub pid: u32,
    pub parent_pid: u32,
    pub session_id: String,
    pub generation: u32,
    pub guard_in_job: Option<bool>,
    #[serde(default)]
    pub spawn_broker: Option<String>,
    pub status: String,
    pub last_seen_unix_ms: u64,
    pub last_seen_local: String,
}

fn default_proof_phase() -> String {
    "prepared".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentionalExitProof {
    pub pid: u32,
    pub session_id: String,
    #[serde(default = "default_proof_phase")]
    pub phase: String,
    pub timestamp_unix_ms: u64,
    pub timestamp_local: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RespawnSessionState {
    pub session_epoch_id: String,
    pub boot_fingerprint: String,
    pub respawn_count: u32,
    pub last_respawn_unix_ms: u64,
    pub last_parent_pid: u32,
}

/// 判断当前进程是否由 `--guardian` 伴生看门狗拉起（同时涵盖轻量 `--silent` 与前台 GUI 复活，用于挂载运行中内核）。
pub fn is_guardian_respawn() -> bool {
    static IS_RESPAWN: OnceLock<bool> = OnceLock::new();
    *IS_RESPAWN.get_or_init(|| std::env::args().any(|arg| arg == "--guardian-respawn"))
}

/// 判断当前进程是否为「轻量/静默托盘形态」的看门狗复活启动（A5：`--guardian-respawn` 且携带 `--silent`）。
pub fn is_guardian_silent_respawn() -> bool {
    static IS_SILENT_RESPAWN: OnceLock<bool> = OnceLock::new();
    *IS_SILENT_RESPAWN.get_or_init(|| {
        let args: Vec<String> = std::env::args().collect();
        args.iter().any(|arg| arg == "--guardian-respawn") && args.iter().any(|arg| arg == "--silent")
    })
}

/// 解析主进程命令行中的 `--guardian-respawn` 元数据（用于新会话首条日志审计留痕与代数继承）。
pub fn parse_respawn_info_from_iter<I, S>(args: I) -> Option<RespawnInfo>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let collected: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if !collected.iter().any(|a| a == "--guardian-respawn") {
        return None;
    }

    let mut generation = 1u32;
    let mut prev_pid = None;
    let mut prev_exit_code = None;
    let silent = collected.iter().any(|a| a == "--silent");

    let mut idx = 0usize;
    while idx < collected.len() {
        match collected[idx].as_str() {
            "--guardian-gen" => {
                if let Some(val) = collected.get(idx + 1).and_then(|v| v.parse::<u32>().ok()) {
                    generation = val;
                    idx += 1;
                }
            }
            "--prev-pid" => {
                if let Some(val) = collected.get(idx + 1).and_then(|v| v.parse::<u32>().ok()) {
                    prev_pid = Some(val);
                    idx += 1;
                }
            }
            "--prev-exit" => {
                if let Some(val) = collected.get(idx + 1).and_then(|v| v.parse::<u32>().ok()) {
                    prev_exit_code = Some(val);
                    idx += 1;
                }
            }
            _ => {}
        }
        idx += 1;
    }

    Some(RespawnInfo {
        generation,
        prev_pid,
        prev_exit_code,
        silent,
    })
}

/// 解析 `--guardian` 伴生看门狗模式的命令行参数。
pub fn parse_guardian_args_from_iter<I, S>(args: I) -> Option<GuardianArgs>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let collected: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if !collected.iter().any(|a| a == "--guardian") {
        return None;
    }

    let mut parent_pid: Option<u32> = None;
    let mut target_exe: Option<PathBuf> = None;
    let mut log_dir: Option<PathBuf> = None;
    let mut guard_dir: Option<PathBuf> = None;
    let mut singleton_port: u16 = 33335;
    let mut generation: u32 = 0;
    let mut session_id: Option<String> = None;
    let mut session_epoch: Option<String> = None;
    let mut broker: Option<String> = None;

    let mut idx = 0usize;
    while idx < collected.len() {
        match collected[idx].as_str() {
            "--parent-pid" => {
                if let Some(val) = collected.get(idx + 1).and_then(|v| v.parse::<u32>().ok()) {
                    parent_pid = Some(val);
                    idx += 1;
                }
            }
            "--target-exe" => {
                if let Some(val) = collected.get(idx + 1).filter(|v| !v.is_empty()) {
                    target_exe = Some(PathBuf::from(val));
                    idx += 1;
                }
            }
            "--log-dir" => {
                if let Some(val) = collected.get(idx + 1) {
                    log_dir = Some(PathBuf::from(val));
                    idx += 1;
                }
            }
            "--guard-dir" => {
                if let Some(val) = collected.get(idx + 1).filter(|v| !v.is_empty()) {
                    guard_dir = Some(PathBuf::from(val));
                    idx += 1;
                }
            }
            "--singleton-port" => {
                if let Some(val) = collected.get(idx + 1).and_then(|v| v.parse::<u16>().ok()) {
                    singleton_port = val;
                    idx += 1;
                }
            }
            "--gen" => {
                if let Some(val) = collected.get(idx + 1).and_then(|v| v.parse::<u32>().ok()) {
                    generation = val;
                    idx += 1;
                }
            }
            "--session-id" => {
                if let Some(val) = collected.get(idx + 1).filter(|v| !v.is_empty()) {
                    session_id = Some(val.clone());
                    idx += 1;
                }
            }
            "--session-epoch" => {
                if let Some(val) = collected.get(idx + 1).filter(|v| !v.is_empty()) {
                    session_epoch = Some(val.clone());
                    idx += 1;
                }
            }
            "--broker" => {
                if let Some(val) = collected.get(idx + 1).filter(|v| !v.is_empty()) {
                    broker = Some(val.clone());
                    idx += 1;
                }
            }
            _ => {}
        }
        idx += 1;
    }

    let resolved_target_exe = target_exe
        .or_else(|| collected.first().filter(|s| !s.is_empty()).map(PathBuf::from))
        .or_else(|| std::env::current_exe().ok())?;

    Some(GuardianArgs {
        parent_pid: parent_pid?,
        target_exe: resolved_target_exe,
        log_dir: log_dir?,
        guard_dir,
        singleton_port,
        generation,
        session_id,
        session_epoch,
        broker,
    })
}

/// 从主进程 CLI 参数解析继承自看门狗的 `--session-epoch`（若缺省返回 `None`）。
pub fn parse_respawn_session_epoch_from_iter<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let collected: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    let mut idx = 0usize;
    while idx < collected.len() {
        if collected[idx] == "--session-epoch"
            && let Some(val) = collected.get(idx + 1).filter(|v| !v.is_empty())
        {
            return Some(val.clone());
        }
        idx += 1;
    }
    None
}

/// 在同一开机与会话纪元内计算下一代自愈计数（A8 / C3）。
///
/// 严格累计至 `MAX_RAPID_RESPAWN_GEN`（3 次）即熔断；严禁依据 `120s` 或 `3600s` 等时间窗自动清零。
/// 仅当发生真实 OS 重启（`boot_fingerprint` 变化）或用户主动执行全新冷启动时才复位预算。
pub const fn compute_next_generation(current_gen: u32) -> Option<u32> {
    let next_gen = current_gen.saturating_add(1);
    if next_gen > MAX_RAPID_RESPAWN_GEN {
        None
    } else {
        Some(next_gen)
    }
}

/// 比较两次操作系统启动指纹是否属于同一次开机窗口（支持 `boot:<unix_secs>` 容许 ±30s 时钟抖动）。
fn boot_fingerprints_match(saved: &str, current: &str) -> bool {
    if let (Some(a), Some(b)) = (saved.strip_prefix("boot:"), current.strip_prefix("boot:"))
        && let (Ok(sec_a), Ok(sec_b)) = (a.parse::<u64>(), b.parse::<u64>())
    {
        return sec_a.abs_diff(sec_b) <= 30;
    }
    saved == current
}

/// 基于持久化会话状态、会话纪元（`session_epoch_id`）与 OS 启动指纹评估是否允许自愈拉起（A8 / C3）。
pub fn evaluate_respawn_budget(
    state: &RespawnSessionState,
    cli_gen: u32,
    expected_epoch_id: &str,
    boot_fingerprint: &str,
) -> Option<u32> {
    if !state.boot_fingerprint.is_empty() && !boot_fingerprints_match(&state.boot_fingerprint, boot_fingerprint) {
        return Some(1);
    }

    if !state.session_epoch_id.is_empty()
        && !expected_epoch_id.is_empty()
        && state.session_epoch_id != expected_epoch_id
    {
        // 状态文件已由用户后续新的干净冷启动接管更新，旧纪元守护链停止操作
        return None;
    }

    let effective_prior = if state.respawn_count == 0 && state.session_epoch_id.is_empty() {
        cli_gen
    } else {
        state.respawn_count.max(cli_gen)
    };

    compute_next_generation(effective_prior)
}

/// 根据伴生看门狗短时间连续异常死亡次数计算重建退避毫秒数（最高封顶 30 秒）。
pub const fn compute_guard_rebuild_backoff_ms(rapid_deaths: u32) -> u64 {
    match rapid_deaths {
        0 => GUARD_REBUILD_BASE_DELAY_MS,
        1 => 2_000,
        2 => 5_000,
        3 => 15_000,
        _ => 30_000,
    }
}

/// 解析物理隔离看门狗驻留目录（优先 `%LOCALAPPDATA%\<APP_ID>\guard`，与 `$INSTDIR` 完全物理隔离）。
pub fn resolve_isolated_guardian_dir() -> PathBuf {
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
        return PathBuf::from(local_app_data)
            .join(crate::utils::dirs::APP_ID)
            .join("guard");
    }
    std::env::temp_dir().join(crate::utils::dirs::APP_ID).join("guard")
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn format_local_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

#[cfg(windows)]
fn atomic_replace_file(src: &Path, dst: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW};
    use windows::core::PCWSTR;

    let src_w: Vec<u16> = src.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let dst_w: Vec<u16> = dst.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;

    for attempt in 0..5u32 {
        // SAFETY: src_w 与 dst_w 均为以空字符结尾的有效 UTF-16 路径切片。
        let res = unsafe { MoveFileExW(PCWSTR(src_w.as_ptr()), PCWSTR(dst_w.as_ptr()), flags) };
        if res.is_ok() {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        let raw = err.raw_os_error();
        // 仅在分享违例/短暂拒绝访问时退避重试，绝不预删现网目标文件（C6）
        if matches!(raw, Some(32 | 5)) && attempt < 4 {
            std::thread::sleep(Duration::from_millis(10u64 << attempt));
            continue;
        }
        return Err(err);
    }
    Err(io::Error::other("atomic_replace_file exhausted retries"))
}

#[cfg(not(windows))]
fn atomic_replace_file(src: &Path, dst: &Path) -> io::Result<()> {
    fs::rename(src, dst)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_vec_pretty(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("state.json");
    let seq = ATOMIC_TEMP_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
    let temp_path = path.with_file_name(format!(
        "{file_name}.tmp.{}.{}.{}",
        std::process::id(),
        current_unix_ms(),
        seq
    ));
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)?;
        f.write_all(&payload)?;
        f.sync_data()?;
    }
    if let Err(err) = atomic_replace_file(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(err);
    }
    Ok(())
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn file_sha256_hex(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_guardian_binary_up_to_date(src_len: u64, src_sha256: &str, dst_path: &Path) -> bool {
    let Ok(dst_meta) = fs::metadata(dst_path) else {
        return false;
    };
    if !dst_meta.is_file() || dst_meta.len() != src_len {
        return false;
    }
    file_sha256_hex(dst_path).is_ok_and(|dst_hash| dst_hash == src_sha256)
}

fn cleanup_stale_guard_staging(guard_dir: &Path) {
    let Ok(entries) = fs::read_dir(guard_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("clash-mini-guard.staging.") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// 将主程序按需同步为异名异目录的独立伴生看门狗可执行文件（`clash-mini-guard.exe`）。
/// 使用字节长度与 SHA-256 摘要强校验内容一致性：一致则零写入直接复用，升级或同长度内容变化均通过 `.staging` 原子替换。
pub fn sync_guardian_binary_to_dir(main_exe: &Path, guard_dir: &Path) -> io::Result<PathBuf> {
    let src_meta = fs::metadata(main_exe)?;
    if !src_meta.is_file() || src_meta.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "main executable metadata is invalid or empty",
        ));
    }
    let src_len = src_meta.len();
    let src_sha256 = file_sha256_hex(main_exe)?;

    fs::create_dir_all(guard_dir)?;
    cleanup_stale_guard_staging(guard_dir);

    let primary_guard = guard_dir.join(ISOLATED_GUARDIAN_BINARY_NAME);
    if is_guardian_binary_up_to_date(src_len, &src_sha256, &primary_guard) {
        return Ok(primary_guard);
    }

    let seq = ATOMIC_TEMP_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
    let staging = guard_dir.join(format!("clash-mini-guard.staging.{}.{seq}.exe", std::process::id()));
    let copied_bytes = fs::copy(main_exe, &staging)?;
    if copied_bytes != src_len {
        let _ = fs::remove_file(&staging);
        return Err(io::Error::new(
            io::ErrorKind::WriteZero,
            "incomplete copy while staging guardian binary",
        ));
    }

    if atomic_replace_file(&staging, &primary_guard).is_ok() {
        return Ok(primary_guard);
    }

    let fallback_guard = guard_dir.join(ISOLATED_GUARDIAN_FALLBACK_NAME);
    if is_guardian_binary_up_to_date(src_len, &src_sha256, &fallback_guard) {
        let _ = fs::remove_file(&staging);
        return Ok(fallback_guard);
    }
    match atomic_replace_file(&staging, &fallback_guard) {
        Ok(()) => Ok(fallback_guard),
        Err(err) => {
            let _ = fs::remove_file(&staging);
            Err(err)
        }
    }
}

/// 对主进程非正常退出码给出可读取证注解。
pub fn format_exit_code_diagnosis(exit_code: u32) -> String {
    match exit_code {
        99 => "99 (0x63: 命中 360加速球/第三方内存清理 TerminateProcess 强杀特征码)".to_string(),
        1 => "1 (0x1: 外部 taskkill / TerminateProcess 强杀或异常终止)".to_string(),
        0xC000_0005 => "0xC0000005 (STATUS_ACCESS_VIOLATION 内存访问违例)".to_string(),
        0xC000_00FD => "0xC00000FD (STATUS_STACK_OVERFLOW 栈溢出)".to_string(),
        0xC000_0409 => "0xC0000409 (STATUS_STACK_BUFFER_OVERRUN / fastfail)".to_string(),
        0xC000_0374 => "0xC0000374 (STATUS_HEAP_CORRUPTION 堆损坏)".to_string(),
        code if code >= 0xC000_0000 => format!("{code} (0x{code:08X}: Windows NTSTATUS 异常终止)"),
        code => format!("{code} (0x{code:X})"),
    }
}

/// 向日志目录的独立看门狗账本 `guardian.log` 同步追加一行结构化取证日志并强制刷盘（A1）。
pub fn append_forensic_log(log_dir: &Path, level: &str, message: &str) {
    append_structured_guardian_log(log_dir, level, "-", std::process::id(), None, 0, "FORENSIC", message);
}

fn emit_guardian_log_fallback(line: &str, err: &io::Error) {
    let trimmed = line.trim_end();
    logging!(
        error,
        Type::System,
        "[Guardian] guardian.log 写入失败 ({err}): {trimmed}"
    );

    let fallback_path = std::env::temp_dir().join("clash-mini-guard-fallback.log");
    if let Ok(mut fallback) = OpenOptions::new().create(true).append(true).open(fallback_path) {
        let fallback_line = format!("[fallback err={err}] {trimmed}\n");
        let _ = fallback.write_all(fallback_line.as_bytes());
        let _ = fallback.sync_data();
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt as _;
        use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
        use windows::core::PCWSTR;

        let msg = format!("[ClashMiniGuardian][log_write_failed={err}] {trimmed}");
        let wide: Vec<u16> = std::ffi::OsStr::new(&msg)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide 为合法以空字符结尾的 UTF-16 诊断文本。
        unsafe {
            OutputDebugStringW(PCWSTR(wide.as_ptr()));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn append_structured_guardian_log(
    log_dir: &Path,
    level: &str,
    session_id: &str,
    parent_pid: u32,
    guard_pid: Option<u32>,
    generation: u32,
    event_code: &str,
    message: &str,
) {
    let now = format_local_timestamp();
    let guard_str = guard_pid.map_or_else(|| "-".to_string(), |p| p.to_string());
    let line = format!(
        "[{now}] {level} | session={session_id} | parent_pid={parent_pid} | guard_pid={guard_str} | gen={generation} | event={event_code} | {message}\n"
    );

    let write_res: io::Result<()> = (|| {
        fs::create_dir_all(log_dir)?;
        let log_file = log_dir.join(GUARDIAN_LOG_FILENAME);
        let mut file = OpenOptions::new().create(true).append(true).open(log_file)?;
        file.write_all(line.as_bytes())?;
        file.sync_data()?;
        Ok(())
    })();

    if let Err(err) = write_res {
        emit_guardian_log_fallback(&line, &err);
    }
}

pub fn write_intentional_exit_proof_to(guard_dir: &Path, proof: &IntentionalExitProof) -> io::Result<()> {
    write_json_atomic(&guard_dir.join(INTENTIONAL_EXIT_FILENAME), proof)
}

pub fn write_intentional_exit_proof_with_retry(guard_dir: &Path, proof: &IntentionalExitProof) -> io::Result<()> {
    let mut last_err = None;
    for attempt in 0..3u32 {
        match write_intentional_exit_proof_to(guard_dir, proof) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_err = Some(err);
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(15));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::other("write intentional exit proof failed")))
}

pub fn read_intentional_exit_proof_from(guard_dir: &Path) -> Option<IntentionalExitProof> {
    read_json_file(&guard_dir.join(INTENTIONAL_EXIT_FILENAME))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentionalProofCheck {
    NotPresent,
    Valid { phase: String, reason: String },
    Rejected { detail: String },
}

/// 严格校验带外主动退出凭证（C1 + C2）：
/// - 必须同时匹配 `proof.pid == parent_pid && proof.session_id == session_id`（`&&` 语义）；
/// - 必须落在相位对应的 TTL 窗内（`prepared` 30s / `confirmed` 120s），超时或损坏主动清理并返回拒绝原因。
pub fn evaluate_intentional_exit_proof(
    guard_dir: &Path,
    parent_pid: u32,
    session_id: &str,
    now_unix_ms: u64,
) -> IntentionalProofCheck {
    let path = guard_dir.join(INTENTIONAL_EXIT_FILENAME);
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return IntentionalProofCheck::NotPresent,
        Err(err) => {
            return IntentionalProofCheck::Rejected {
                detail: format!("读取退出凭证文件失败: {err}"),
            };
        }
    };

    let proof: IntentionalExitProof = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(err) => {
            let _ = fs::remove_file(&path);
            return IntentionalProofCheck::Rejected {
                detail: format!("退出凭证 JSON 损坏并已清理: {err}"),
            };
        }
    };

    if proof.pid != parent_pid || proof.session_id != session_id {
        return IntentionalProofCheck::Rejected {
            detail: format!(
                "退出凭证身份不匹配 (expected pid={parent_pid}/session={session_id}, got pid={}/session={})",
                proof.pid, proof.session_id
            ),
        };
    }

    if proof.timestamp_unix_ms > now_unix_ms.saturating_add(5_000) {
        let _ = fs::remove_file(&path);
        return IntentionalProofCheck::Rejected {
            detail: format!(
                "退出凭证时间戳超前异常并已清理 (proof_ts={}, now={now_unix_ms})",
                proof.timestamp_unix_ms
            ),
        };
    }

    let max_ttl_ms = if proof.phase == "confirmed" {
        INTENTIONAL_EXIT_CONFIRMED_TTL_MS
    } else {
        INTENTIONAL_EXIT_PROOF_TTL_MS
    };
    let age_ms = now_unix_ms.saturating_sub(proof.timestamp_unix_ms);
    if age_ms > max_ttl_ms {
        let _ = fs::remove_file(&path);
        return IntentionalProofCheck::Rejected {
            detail: format!(
                "退出凭证已过 TTL 有效期并清理 (phase={}, age_ms={age_ms} > ttl_ms={max_ttl_ms}, reason={})",
                proof.phase, proof.reason
            ),
        };
    }

    IntentionalProofCheck::Valid {
        phase: proof.phase,
        reason: proof.reason,
    }
}

pub fn clear_intentional_exit_proof_in(guard_dir: &Path) {
    let path = guard_dir.join(INTENTIONAL_EXIT_FILENAME);
    if fs::remove_file(&path).is_err() && path.exists() {
        std::thread::sleep(Duration::from_millis(15));
        let _ = fs::remove_file(&path);
    }
}

pub fn write_main_heartbeat_to(guard_dir: &Path, hb: &MainHeartbeat) -> io::Result<()> {
    write_json_atomic(&guard_dir.join(MAIN_HEARTBEAT_FILENAME), hb)
}

pub fn write_guard_heartbeat_to(guard_dir: &Path, hb: &GuardHeartbeat) -> io::Result<()> {
    write_json_atomic(&guard_dir.join(GUARD_HEARTBEAT_FILENAME), hb)
}

pub fn read_respawn_state_checked(guard_dir: &Path) -> Result<RespawnSessionState, String> {
    let path = guard_dir.join(RESPAWN_STATE_FILENAME);
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<RespawnSessionState>(&bytes)
            .map_err(|err| format!("respawn_state.json 解析损坏: {err}")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(RespawnSessionState::default()),
        Err(err) => Err(format!("读取 respawn_state.json 失败: {err}")),
    }
}

pub fn read_respawn_state_from(guard_dir: &Path) -> RespawnSessionState {
    read_json_file(&guard_dir.join(RESPAWN_STATE_FILENAME)).unwrap_or_default()
}

pub fn reset_respawn_state_for_clean_start_in(
    guard_dir: &Path,
    log_dir: Option<&Path>,
    session_id: &str,
    boot_fingerprint: &str,
) {
    let previous = read_respawn_state_from(guard_dir);
    let my_pid = std::process::id();
    if previous.respawn_count > 0
        && let Some(dir) = log_dir
    {
        append_structured_guardian_log(
            dir,
            "INFO",
            session_id,
            my_pid,
            None,
            0,
            "RESPAWN_BUDGET_RESET_ON_CLEAN_START",
            &format!(
                "干净冷启动复位前序累计复活计数 (cleared_count={}, old_epoch={}, old_boot={})",
                previous.respawn_count, previous.session_epoch_id, previous.boot_fingerprint
            ),
        );
    }

    let state = RespawnSessionState {
        session_epoch_id: session_id.to_string(),
        boot_fingerprint: boot_fingerprint.to_string(),
        respawn_count: 0,
        last_respawn_unix_ms: 0,
        last_parent_pid: my_pid,
    };
    if let Err(err) = write_json_atomic(&guard_dir.join(RESPAWN_STATE_FILENAME), &state)
        && let Some(dir) = log_dir
    {
        append_structured_guardian_log(
            dir,
            "WARN",
            session_id,
            my_pid,
            None,
            0,
            "RESPAWN_STATE_RESET_FAILED",
            &format!("干净启动重置 respawn_state.json 失败: {err}"),
        );
    }
}

/// Fail-closed 评估并在 `commit=true` 时持久化复活代数（C3 + C4）：
/// 状态文件读取损坏或写入失败一律返回 `Err`，调用方必须放弃拉起。
#[allow(clippy::too_many_arguments)]
pub fn evaluate_and_record_respawn_in(
    guard_dir: &Path,
    epoch_id: &str,
    cli_gen: u32,
    parent_pid: u32,
    now_unix_ms: u64,
    boot_fingerprint: &str,
    commit: bool,
) -> Result<Option<u32>, String> {
    let state = read_respawn_state_checked(guard_dir)?;
    let Some(next_gen) = evaluate_respawn_budget(&state, cli_gen, epoch_id, boot_fingerprint) else {
        return Ok(None);
    };
    if commit {
        let updated = RespawnSessionState {
            session_epoch_id: if !boot_fingerprints_match(&state.boot_fingerprint, boot_fingerprint)
                || state.session_epoch_id.is_empty()
            {
                epoch_id.to_string()
            } else {
                state.session_epoch_id
            },
            boot_fingerprint: boot_fingerprint.to_string(),
            respawn_count: next_gen,
            last_respawn_unix_ms: now_unix_ms,
            last_parent_pid: parent_pid,
        };
        write_json_atomic(&guard_dir.join(RESPAWN_STATE_FILENAME), &updated)
            .map_err(|err| format!("写入 respawn_state.json 失败 (Fail-Closed 拒绝拉起): {err}"))?;
    }
    Ok(Some(next_gen))
}

/// 检查当前系统中是否有安装/卸载器正在运行，避免升级覆盖或卸载期间与安装器抢拉进程。
fn is_installer_or_uninstaller_running() -> bool {
    let system = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
    );
    for process in system.processes().values() {
        let name = process.name().to_string_lossy().to_ascii_lowercase();
        if name == "uninstall.exe"
            || name == "au_.exe"
            || name.starts_with("clash.mini_")
            || name.starts_with("clash-mini-setup")
            || name.starts_with("clashwin")
            || name.contains("un_a.exe")
        {
            return true;
        }
    }
    false
}

/// 在 `main()` 最顶部（初始化 Tokio / Tauri / WebView2 之前）拦截 `--guardian` 模式。
/// 若当前为看门狗伴生进程，执行阻塞守护循环并返回退出码 `Some(0)`；否则立即返回 `None`。
pub fn maybe_run_guardian_mode() -> Option<i32> {
    let args = parse_guardian_args_from_iter(std::env::args())?;
    #[cfg(windows)]
    {
        win32_impl::run_guardian_loop(&args);
    }
    Some(0)
}

/// 在主进程 UI 主线程创建常驻零 WebView 哨兵宿主窗口对（防第三方加速球无窗口残留误杀）。
pub fn init_resident_sentinel_window() {
    #[cfg(windows)]
    win32_impl::init_resident_sentinel_window();
}

/// 在主进程初始化完成后创建同步内核事件并启动双向互锁伴生看门狗监督线程。
pub fn start_guardian_companion() {
    if cfg!(test) || std::env::var("CLASH_MINI_DISABLE_GUARDIAN").as_deref() == Ok("1") {
        return;
    }

    if let Some(info) = parse_respawn_info_from_iter(std::env::args()) {
        let prev_exit_desc = info
            .prev_exit_code
            .map_or_else(|| "未知".to_string(), format_exit_code_diagnosis);
        logging!(
            warn,
            Type::System,
            "[Guardian] 当前实例已由伴生看门狗自愈拉起（第 {} 代，形态={}，前序 PID={:?}，前序退出码={}），复用运行中内核恢复防断流守护",
            info.generation,
            if info.silent { "silent" } else { "gui" },
            info.prev_pid,
            prev_exit_desc
        );
    }

    #[cfg(windows)]
    {
        let Ok(log_dir) = crate::utils::dirs::app_logs_dir() else {
            return;
        };
        let singleton_port = crate::config::IVerge::get_singleton_port();
        let current_gen = parse_respawn_info_from_iter(std::env::args()).map_or(0, |r| r.generation);
        let inherited_epoch = parse_respawn_session_epoch_from_iter(std::env::args());
        win32_impl::init_session_and_spawn_companion(&log_dir, singleton_port, current_gen, inherited_epoch.as_deref());
    }
}

/// 幂等确保伴生看门狗监督线程处于运行态（A6：由后台巡检周期性调用，避免一次性退场导致会话永久失防）。
pub fn ensure_guardian_supervisor() {
    if cfg!(test) || std::env::var("CLASH_MINI_DISABLE_GUARDIAN").as_deref() == Ok("1") {
        return;
    }
    #[cfg(windows)]
    win32_impl::ensure_guardian_supervisor();
}

/// 若距离上次刷新已达 15 秒，则落盘宿主独立诊断心跳（A2：仅作排障证据，不参与复活判决）。
pub fn record_main_heartbeat_if_due() {
    if cfg!(test) || std::env::var("CLASH_MINI_DISABLE_GUARDIAN").as_deref() == Ok("1") {
        return;
    }
    #[cfg(windows)]
    win32_impl::record_main_heartbeat_if_due();
}

/// 同步更新看门狗感知的「主进程是否处于后台托盘/轻量驻留模式」内核事件状态（A5：含回读一致性核对）。
pub fn set_lightweight_mode_signal(in_lightweight: bool) {
    #[cfg(windows)]
    win32_impl::set_lightweight_mode_signal(in_lightweight);
}

/// 在用户显式点击退出或收到系统关机/注销信号的第一微秒通知看门狗与本地监督线程正常退场（`prepared` 阶段）。
pub fn notify_intentional_exit() {
    notify_intentional_exit_with_reason("intentional_exit");
}

/// 带原因码落盘 `prepared` 阶段主动退出凭证（30s TTL，失败重试）并置位 `exit_event`（A7 / C1）。
pub fn notify_intentional_exit_with_reason(reason: &str) {
    #[cfg(windows)]
    win32_impl::write_intentional_exit_phase("prepared", reason);
    #[cfg(not(windows))]
    let _ = reason;
}

/// 在 `RunEvent::Exit` 确认进入最终进程终结阶段时升级写入 `confirmed` 退出凭证并置位 `exit_event`（A7 / C1）。
pub fn confirm_intentional_exit_with_reason(reason: &str) {
    #[cfg(windows)]
    win32_impl::write_intentional_exit_phase("confirmed", reason);
    #[cfg(not(windows))]
    let _ = reason;
}

/// 当 `ExitRequested` 被取消或阻止退出时，同步撤销带外退出凭证、`ResetEvent` 并确认监督重新武装（A7）。
pub fn cancel_intentional_exit(reason: &str) {
    #[cfg(windows)]
    win32_impl::cancel_intentional_exit(reason);
    #[cfg(not(windows))]
    let _ = reason;
}

/// 返回看门狗与隔离状态简报（供系统诊断报告展示，A3 / A6）。
pub fn get_diagnostic_summary() -> String {
    #[cfg(windows)]
    {
        win32_impl::get_diagnostic_summary()
    }
    #[cfg(not(windows))]
    {
        "非 Windows 平台无看门狗".to_string()
    }
}

#[cfg(windows)]
mod win32_impl {
    use super::{
        BROKER_BREAKAWAY_JOB, BROKER_SAME_JOB_DEGRADED, BROKER_TASK_SCHEDULER_COM, BROKER_WMI_SESSION0, Command,
        Duration, GUARD_RAPID_DEATH_WINDOW_SECS, GuardHeartbeat, GuardianArgs, HEARTBEAT_INTERVAL_MS,
        INSTALLER_WAIT_MAX_SECS, INTENTIONAL_EXIT_FILENAME, IntentionalExitProof, IntentionalProofCheck,
        MAX_RAPID_RESPAWN_GEN, MainHeartbeat, Path, PathBuf, RESPAWN_SAFETY_DELAY_MS, TcpListener,
        append_structured_guardian_log, clear_intentional_exit_proof_in, compute_guard_rebuild_backoff_ms,
        current_unix_ms, evaluate_and_record_respawn_in, evaluate_intentional_exit_proof, format_exit_code_diagnosis,
        format_local_timestamp, is_guardian_respawn, is_installer_or_uninstaller_running, read_respawn_state_from,
        reset_respawn_state_for_clean_start_in, resolve_isolated_guardian_dir, sync_guardian_binary_to_dir,
        write_guard_heartbeat_to, write_intentional_exit_proof_with_retry, write_main_heartbeat_to,
    };
    use clash_verge_logging::{Type, logging};
    use std::{
        ffi::{OsStr, c_void},
        mem::ManuallyDrop,
        os::windows::{ffi::OsStrExt as _, io::AsRawHandle as _, process::CommandExt as _},
        process::Child,
        sync::{
            Mutex, Once, OnceLock,
            atomic::{AtomicBool, AtomicI8, AtomicIsize, AtomicU32, AtomicU64, Ordering},
        },
        time::Instant,
    };

    type Handle = *mut c_void;
    type Hwnd = *mut c_void;

    const SYNCHRONIZE: u32 = 0x0010_0000;
    const EVENT_MODIFY_STATE: u32 = 0x0002;
    const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const INFINITE: u32 = 0xFFFF_FFFF;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 0x0000_0102;
    const WAIT_FAILED: u32 = 0xFFFF_FFFF;

    const WS_POPUP: u32 = 0x8000_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
    const LWA_ALPHA: u32 = 0x0000_0002;
    const SPI_GETWORKAREA: u32 = 0x0030;
    const WM_NCHITTEST: u32 = 0x0084;
    const HTTRANSPARENT: isize = -1;
    const QS_ALLINPUT: u32 = 0x04FF;
    const PM_REMOVE: u32 = 0x0001;
    const SM_SHUTTINGDOWN: i32 = 0x2000;

    /// `1/255`（约 0.39% 不透明度）：对人眼 100% 不可见，同时令 `GetLayeredWindowAttributes` 返回非零 `bAlpha`。
    pub(super) const SENTINEL_VISIBLE_ALPHA: u8 = 1;

    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    impl FileTime {
        const fn to_u64(self) -> u64 {
            ((self.high as u64) << 32) | (self.low as u64)
        }
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        w_param: usize,
        l_param: isize,
        time: u32,
        pt: Point,
    }

    impl Default for Msg {
        fn default() -> Self {
            Self {
                hwnd: std::ptr::null_mut(),
                message: 0,
                w_param: 0,
                l_param: 0,
                time: 0,
                pt: Point::default(),
            }
        }
    }

    #[cfg(target_pointer_width = "64")]
    const _: () = assert!(std::mem::size_of::<Msg>() == 48);

    #[repr(C)]
    struct WndClassExW {
        cb_size: u32,
        style: u32,
        lpfn_wnd_proc: unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize,
        cb_cls_extra: i32,
        cb_wnd_extra: i32,
        h_instance: Handle,
        h_icon: Handle,
        h_cursor: Handle,
        hbr_background: Handle,
        lpsz_menu_name: *const u16,
        lpsz_class_name: *const u16,
        h_icon_sm: Handle,
    }

    unsafe extern "system" {
        fn GetCurrentProcess() -> Handle;
        fn GetCurrentProcessId() -> u32;
        fn GetLastError() -> u32;
        fn GetTickCount64() -> u64;
        fn GetModuleHandleW(lp_module_name: *const u16) -> Handle;
        fn IsProcessInJob(process_handle: Handle, job_handle: Handle, result: *mut i32) -> i32;
        fn CreateEventW(
            lp_event_attributes: *const c_void,
            b_manual_reset: i32,
            b_initial_state: i32,
            lp_name: *const u16,
        ) -> Handle;
        fn OpenEventW(dw_desired_access: u32, b_inherit_handle: i32, lp_name: *const u16) -> Handle;
        fn SetEvent(h_event: Handle) -> i32;
        fn ResetEvent(h_event: Handle) -> i32;
        fn OpenProcess(dw_desired_access: u32, b_inherit_handle: i32, dw_process_id: u32) -> Handle;
        fn WaitForSingleObject(h_handle: Handle, dw_milliseconds: u32) -> u32;
        fn WaitForMultipleObjects(
            n_count: u32,
            lp_handles: *const Handle,
            b_wait_all: i32,
            dw_milliseconds: u32,
        ) -> u32;
        fn MsgWaitForMultipleObjects(
            n_count: u32,
            p_handles: *const Handle,
            f_wait_all: i32,
            dw_milliseconds: u32,
            dw_wake_mask: u32,
        ) -> u32;
        fn GetExitCodeProcess(h_process: Handle, lp_exit_code: *mut u32) -> i32;
        fn GetProcessTimes(
            h_process: Handle,
            lp_creation_time: *mut FileTime,
            lp_exit_time: *mut FileTime,
            lp_kernel_time: *mut FileTime,
            lp_user_time: *mut FileTime,
        ) -> i32;
        fn CloseHandle(h_object: Handle) -> i32;

        fn RegisterClassExW(lp_wnd_class: *const WndClassExW) -> u16;
        fn CreateWindowExW(
            dw_ex_style: u32,
            lp_class_name: *const u16,
            lp_window_name: *const u16,
            dw_style: u32,
            x: i32,
            y: i32,
            n_width: i32,
            n_height: i32,
            h_wnd_parent: Hwnd,
            h_menu: Handle,
            h_instance: Handle,
            lp_param: *const c_void,
        ) -> Hwnd;
        fn DestroyWindow(h_wnd: Hwnd) -> i32;
        fn DefWindowProcW(h_wnd: Hwnd, msg: u32, w_param: usize, l_param: isize) -> isize;
        fn SetLayeredWindowAttributes(hwnd: Hwnd, cr_key: u32, b_alpha: u8, dw_flags: u32) -> i32;
        fn SystemParametersInfoW(ui_action: u32, ui_param: u32, pv_param: *mut c_void, f_win_ini: u32) -> i32;
        fn PeekMessageW(
            lp_msg: *mut Msg,
            h_wnd: Hwnd,
            w_msg_filter_min: u32,
            w_msg_filter_max: u32,
            w_remove_msg: u32,
        ) -> i32;
        fn TranslateMessage(lp_msg: *const Msg) -> i32;
        fn DispatchMessageW(lp_msg: *const Msg) -> isize;
        fn GetSystemMetrics(n_index: i32) -> i32;
    }

    pub(super) struct GuardianSession {
        pub(super) pid: u32,
        pub(super) session_id: String,
        pub(super) session_epoch: String,
        pub(super) log_dir: PathBuf,
        pub(super) guard_dir: PathBuf,
        pub(super) singleton_port: u16,
        pub(super) initial_gen: u32,
        pub(super) exit_event: isize,
        pub(super) lightweight_event: isize,
        pub(super) parent_in_job: Option<bool>,
        pub(super) breakaway_degraded: AtomicBool,
        pub(super) guard_in_job: AtomicI8,
        pub(super) spawn_broker: Mutex<Option<String>>,
        pub(super) current_guard_pid: AtomicU32,
        pub(super) last_heartbeat_unix_ms: AtomicU64,
    }

    struct SupervisorRunningGuard;

    impl Drop for SupervisorRunningGuard {
        fn drop(&mut self) {
            GUARDIAN_SUPERVISOR_STARTED.store(false, Ordering::Release);
        }
    }

    static GUARDIAN_SESSION: OnceLock<GuardianSession> = OnceLock::new();
    static GUARDIAN_SUPERVISOR_STARTED: AtomicBool = AtomicBool::new(false);
    static SENTINEL_OWNER_HWND: AtomicIsize = AtomicIsize::new(0);
    static SENTINEL_VISIBLE_HWND: AtomicIsize = AtomicIsize::new(0);

    pub(super) fn to_wide_null(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    pub(super) fn exit_event_name(pid: u32) -> String {
        format!("Local\\ClashMiniIntentionalExit_{pid}")
    }

    pub(super) fn lightweight_event_name(pid: u32) -> String {
        format!("Local\\ClashMiniInLightweight_{pid}")
    }

    pub(super) fn query_handle_in_job(process_handle: Handle) -> Option<bool> {
        let mut in_job: i32 = 0;
        // SAFETY: process_handle 为有效进程句柄，in_job 指向栈上有效 i32。
        let ok = unsafe { IsProcessInJob(process_handle, std::ptr::null_mut(), &raw mut in_job) };
        if ok == 0 { None } else { Some(in_job != 0) }
    }

    pub(super) fn query_current_process_in_job() -> Option<bool> {
        // SAFETY: GetCurrentProcess 返回伪句柄，合法传入 IsProcessInJob。
        query_handle_in_job(unsafe { GetCurrentProcess() })
    }

    fn current_boot_fingerprint() -> String {
        let now_secs = current_unix_ms() / 1000;
        // SAFETY: GetTickCount64 无前置状态要求。
        let up_secs = unsafe { GetTickCount64() } / 1000;
        format!("boot:{}", now_secs.saturating_sub(up_secs))
    }

    /// 带重试与回读一致性的命名事件置位/复位（A5 / A7），失败返回 `GetLastError()`。
    fn signal_event_with_retry(event_handle: isize, signaled: bool) -> Result<(), u32> {
        if event_handle == 0 {
            return Err(6); // ERROR_INVALID_HANDLE
        }
        let handle = event_handle as Handle;
        let expected_wait = if signaled { WAIT_OBJECT_0 } else { WAIT_TIMEOUT };
        let mut last_err = 0u32;

        for attempt in 0..3u8 {
            // SAFETY: handle 为当前会话生命周期常驻的有效手动重置事件句柄。
            let ok = unsafe { if signaled { SetEvent(handle) } else { ResetEvent(handle) } };
            if ok == 0 {
                // SAFETY: 紧随 Win32 API 失败调用 GetLastError。
                last_err = unsafe { GetLastError() };
            } else {
                // SAFETY: 0ms 非阻塞回读事件内核态进行一致性验证。
                let readback = unsafe { WaitForSingleObject(handle, 0) };
                if readback == expected_wait {
                    return Ok(());
                }
                last_err = if readback == WAIT_FAILED {
                    // SAFETY: 紧随 WaitForSingleObject 失败读取 GetLastError。
                    unsafe { GetLastError() }
                } else {
                    u32::MAX
                };
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        Err(last_err)
    }

    fn write_main_heartbeat_snapshot(session: &GuardianSession, now_ms: u64) {
        let guard_pid = match session.current_guard_pid.load(Ordering::Acquire) {
            0 => None,
            pid => Some(pid),
        };
        let spawn_broker = session.spawn_broker.lock().ok().and_then(|g| g.clone());
        let hb = MainHeartbeat {
            pid: session.pid,
            session_id: session.session_id.clone(),
            in_lightweight: crate::module::lightweight::is_in_lightweight_mode(),
            is_exiting: crate::core::handle::Handle::global().is_exiting(),
            parent_in_job: session.parent_in_job,
            breakaway_degraded: session.breakaway_degraded.load(Ordering::Acquire),
            spawn_broker,
            guard_pid,
            last_seen_unix_ms: now_ms,
            last_seen_local: format_local_timestamp(),
        };
        session.last_heartbeat_unix_ms.store(now_ms, Ordering::Release);
        let _ = write_main_heartbeat_to(&session.guard_dir, &hb);
    }

    pub(super) fn record_main_heartbeat_if_due() {
        let Some(session) = GUARDIAN_SESSION.get() else {
            return;
        };
        let now_ms = current_unix_ms();
        let last = session.last_heartbeat_unix_ms.load(Ordering::Acquire);
        if now_ms.saturating_sub(last) >= HEARTBEAT_INTERVAL_MS {
            write_main_heartbeat_snapshot(session, now_ms);
        }
    }

    unsafe extern "system" fn sentinel_wnd_proc(hwnd: Hwnd, msg: u32, w_param: usize, l_param: isize) -> isize {
        if msg == WM_NCHITTEST {
            return HTTRANSPARENT;
        }
        // SAFETY: 标准默认窗口过程，转发所有消息以保证 IsHungAppWindow 恒为 FALSE。
        unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
    }

    fn query_primary_workarea_origin() -> (i32, i32) {
        let mut rect = Rect::default();
        // SAFETY: SPI_GETWORKAREA 写入标准 16 字节 RECT 结构，无副作用。
        let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&raw mut rect).cast::<c_void>(), 0) };
        if ok != 0 { (rect.left, rect.top) } else { (0, 0) }
    }

    /// 创建一对原生零渲染宿主窗口：
    /// - 隐藏 `WS_POPUP` Owner 位于屏外（消除任务栏按钮与 Alt+Tab 项）；
    /// - 归属其下的 `1x1` `WS_POPUP | WS_VISIBLE` 哨兵窗口落在主屏工作区左上角，`bAlpha = 1` 鼠标完全穿透且满足在屏可见窗口判据。
    pub(super) fn create_sentinel_window_pair(class_name_str: &str, title_str: &str) -> Option<(Hwnd, Hwnd)> {
        let class_name = to_wide_null(class_name_str);
        let owner_title = to_wide_null("ClashMiniResidentOwner");
        let sentinel_title = to_wide_null(title_str);
        let (work_x, work_y) = query_primary_workarea_origin();

        // SAFETY: 在当前线程注册自定义轻量窗口类并创建工作区 1x1 穿透归属窗口对。
        unsafe {
            let h_instance = GetModuleHandleW(std::ptr::null());
            let wc = WndClassExW {
                cb_size: std::mem::size_of::<WndClassExW>() as u32,
                style: 0,
                lpfn_wnd_proc: sentinel_wnd_proc,
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance,
                h_icon: std::ptr::null_mut(),
                h_cursor: std::ptr::null_mut(),
                hbr_background: std::ptr::null_mut(),
                lpsz_menu_name: std::ptr::null(),
                lpsz_class_name: class_name.as_ptr(),
                h_icon_sm: std::ptr::null_mut(),
            };
            let _ = RegisterClassExW(&raw const wc);

            let owner = CreateWindowExW(
                0,
                class_name.as_ptr(),
                owner_title.as_ptr(),
                WS_POPUP,
                -32000,
                -32000,
                1,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                h_instance,
                std::ptr::null(),
            );
            if owner.is_null() {
                return None;
            }

            let sentinel = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                sentinel_title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                work_x,
                work_y,
                1,
                1,
                owner,
                std::ptr::null_mut(),
                h_instance,
                std::ptr::null(),
            );
            if sentinel.is_null() {
                let _ = DestroyWindow(owner);
                return None;
            }

            let _ = SetLayeredWindowAttributes(sentinel, 0, SENTINEL_VISIBLE_ALPHA, LWA_ALPHA);
            Some((owner, sentinel))
        }
    }

    fn destroy_sentinel_window_pair(pair: Option<(Hwnd, Hwnd)>) {
        if let Some((owner, sentinel)) = pair {
            // SAFETY: 销毁当前线程自己创建的哨兵窗口与 Owner 窗口句柄。
            unsafe {
                if !sentinel.is_null() {
                    let _ = DestroyWindow(sentinel);
                }
                if !owner.is_null() {
                    let _ = DestroyWindow(owner);
                }
            }
        }
    }

    pub(super) fn init_resident_sentinel_window() {
        if SENTINEL_VISIBLE_HWND.load(Ordering::Acquire) != 0 {
            return;
        }
        if let Some((owner, sentinel)) = create_sentinel_window_pair("ClashMiniResidentHost", "Clash Mini Tray Host") {
            SENTINEL_OWNER_HWND.store(owner as isize, Ordering::Release);
            SENTINEL_VISIBLE_HWND.store(sentinel as isize, Ordering::Release);
            logging!(
                info,
                Type::Window,
                "[Guardian] 已挂载常驻零渲染哨兵宿主窗口 (sentinel=0x{:X})",
                sentinel as usize
            );
        } else {
            logging!(warn, Type::Window, "[Guardian] 创建常驻哨兵宿主窗口失败");
        }
    }

    fn is_exit_event_signaled(exit_event: isize) -> bool {
        if exit_event == 0 {
            return false;
        }
        // SAFETY: exit_event 为当前进程生命周期内常驻的有效事件句柄。
        unsafe { WaitForSingleObject(exit_event as Handle, 0) == WAIT_OBJECT_0 }
    }

    /// 在指定毫秒内等待主动退出事件；若收到退出事件或应用进入退出态则立即返回 `true`（0ms 唤醒）。
    fn wait_exit_or_timeout(exit_event: isize, timeout_ms: u64) -> bool {
        let clamped_ms = timeout_ms.min(u64::from(INFINITE - 1)) as u32;
        if exit_event != 0 {
            // SAFETY: exit_event 为有效手动重置事件句柄。
            let res = unsafe { WaitForSingleObject(exit_event as Handle, clamped_ms) };
            if res == WAIT_OBJECT_0 {
                return true;
            }
        } else {
            std::thread::sleep(Duration::from_millis(timeout_ms));
        }
        crate::core::handle::Handle::global().is_exiting() || is_system_shutting_down()
    }

    fn resolve_guardian_executable(
        main_exe: &Path,
        guard_dir: &Path,
        session: &GuardianSession,
        generation: u32,
    ) -> PathBuf {
        match sync_guardian_binary_to_dir(main_exe, guard_dir) {
            Ok(isolated) => isolated,
            Err(err) => {
                logging!(
                    warn,
                    Type::Setup,
                    "[Guardian] 同步物理隔离看门狗二进制失败，降级使用主程序路径 ({}): {}",
                    main_exe.display(),
                    err
                );
                append_structured_guardian_log(
                    &session.log_dir,
                    "WARN",
                    &session.session_id,
                    session.pid,
                    None,
                    generation,
                    "SUPERVISOR_BINARY_SYNC_DEGRADED",
                    &format!(
                        "同步隔离二进制至 {} 失败，降级直接拉起主程序 ({}): {}",
                        guard_dir.display(),
                        main_exe.display(),
                        err
                    ),
                );
                main_exe.to_path_buf()
            }
        }
    }

    struct ComInitGuard {
        should_uninit: bool,
    }

    impl ComInitGuard {
        fn enter() -> Result<Self, String> {
            use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
            use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

            // SAFETY: 当前线程按需求初始化 COM 套间，仅在 S_OK / S_FALSE 时配对 CoUninitialize。
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if hr.is_ok() {
                Ok(Self { should_uninit: true })
            } else if hr == RPC_E_CHANGED_MODE {
                Ok(Self { should_uninit: false })
            } else {
                Err(format!("CoInitializeEx failed: {hr}"))
            }
        }
    }

    impl Drop for ComInitGuard {
        fn drop(&mut self) {
            if self.should_uninit {
                use windows::Win32::System::Com::CoUninitialize;
                // SAFETY: 仅在当前线程成功 CoInitializeEx 时成对释放 COM 套间。
                unsafe { CoUninitialize() };
            }
        }
    }

    fn ensure_com_security() {
        static COM_SECURITY_ONCE: Once = Once::new();
        COM_SECURITY_ONCE.call_once(|| {
            use windows::Win32::System::Com::{
                CoInitializeSecurity, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
            };
            // SAFETY: 进程生命周期内仅初始化一次默认 COM 安全上下文（已初始化时返回 RPC_E_TOO_LATE 可忽略）。
            unsafe {
                let _ = CoInitializeSecurity(
                    None,
                    -1,
                    None,
                    None,
                    RPC_C_AUTHN_LEVEL_CALL,
                    RPC_C_IMP_LEVEL_IMPERSONATE,
                    None,
                    EOAC_NONE,
                    None,
                );
            }
        });
    }

    #[derive(Default)]
    struct OwnedVariant(windows::Win32::System::Variant::VARIANT);

    impl Drop for OwnedVariant {
        fn drop(&mut self) {
            use windows::Win32::System::Variant::VariantClear;
            // SAFETY: self.0 始终为有效或 zero-init 的 VARIANT。
            unsafe {
                let _ = VariantClear(&raw mut self.0);
            }
        }
    }

    impl OwnedVariant {
        fn from_bstr(s: &str) -> Self {
            use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_BSTR};
            use windows::core::BSTR;

            Self(VARIANT {
                Anonymous: VARIANT_0 {
                    Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                        vt: VT_BSTR,
                        wReserved1: 0,
                        wReserved2: 0,
                        wReserved3: 0,
                        Anonymous: VARIANT_0_0_0 {
                            bstrVal: ManuallyDrop::new(BSTR::from(s)),
                        },
                    }),
                },
            })
        }

        const fn from_i32(v: i32) -> Self {
            use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};

            Self(VARIANT {
                Anonymous: VARIANT_0 {
                    Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                        vt: VT_I4,
                        wReserved1: 0,
                        wReserved2: 0,
                        wReserved3: 0,
                        Anonymous: VARIANT_0_0_0 { lVal: v },
                    }),
                },
            })
        }

        const fn from_unknown(unk: windows::core::IUnknown) -> Self {
            use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_UNKNOWN};

            Self(VARIANT {
                Anonymous: VARIANT_0 {
                    Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                        vt: VT_UNKNOWN,
                        wReserved1: 0,
                        wReserved2: 0,
                        wReserved3: 0,
                        Anonymous: VARIANT_0_0_0 {
                            punkVal: ManuallyDrop::new(Some(unk)),
                        },
                    }),
                },
            })
        }
    }

    fn escape_windows_arg(arg: &str) -> String {
        if !arg.is_empty() && !arg.chars().any(|c| c == ' ' || c == '\t' || c == '"') {
            return arg.to_string();
        }
        let mut out = String::with_capacity(arg.len() + 2);
        out.push('"');
        let mut backslashes = 0usize;
        for ch in arg.chars() {
            match ch {
                '\\' => backslashes += 1,
                '"' => {
                    for _ in 0..backslashes * 2 + 1 {
                        out.push('\\');
                    }
                    out.push('"');
                    backslashes = 0;
                }
                _ => {
                    for _ in 0..backslashes {
                        out.push('\\');
                    }
                    backslashes = 0;
                    out.push(ch);
                }
            }
        }
        for _ in 0..backslashes * 2 {
            out.push('\\');
        }
        out.push('"');
        out
    }

    fn build_quoted_arguments(args: &[String]) -> String {
        args.iter().map(|a| escape_windows_arg(a)).collect::<Vec<_>>().join(" ")
    }

    fn build_quoted_command_line(exe: &Path, args: &[String]) -> String {
        let exe_quoted = escape_windows_arg(&exe.to_string_lossy());
        if args.is_empty() {
            exe_quoted
        } else {
            format!("{exe_quoted} {}", build_quoted_arguments(args))
        }
    }

    fn build_wmi_hidden_startup_info(services: &windows::Win32::System::Wmi::IWbemServices) -> Option<OwnedVariant> {
        use windows::Win32::System::Wmi::{IWbemClassObject, WBEM_GENERIC_FLAG_TYPE};
        use windows::core::{BSTR, Interface as _, w};

        let mut startup_cls: Option<IWbemClassObject> = None;
        // SAFETY: services 已连接并通过 CoSetProxyBlanket 设置认证上下文。
        unsafe {
            services
                .GetObject(
                    &BSTR::from("Win32_ProcessStartup"),
                    WBEM_GENERIC_FLAG_TYPE(0),
                    None,
                    Some(&raw mut startup_cls),
                    None,
                )
                .ok()?;
            let startup_inst = startup_cls?.SpawnInstance(0).ok()?;
            let show_hide = OwnedVariant::from_i32(0); // SW_HIDE = 0
            startup_inst.Put(w!("ShowWindow"), 0, &raw const show_hide.0, 0).ok()?;
            let unk = startup_inst.cast::<windows::core::IUnknown>().ok()?;
            Some(OwnedVariant::from_unknown(unk))
        }
    }

    /// A6 一级代理：通过 Session-0 WMI 提供程序（`WmiPrvSE.exe`）执行 `Win32_Process::Create`。
    /// 无需提升权限亦无控制台闪烁，子进程天然继承交互用户桌面并脱离父进程树与父 Job Object。
    fn wmi_spawn_process(
        exe: &Path,
        args: &[String],
        work_dir: Option<&Path>,
        hide_window: bool,
    ) -> Result<u32, String> {
        use windows::Win32::System::Com::{
            CLSCTX_INPROC_SERVER, CoCreateInstance, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
        };
        use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
        use windows::Win32::System::Variant::VariantToUInt32;
        use windows::Win32::System::Wmi::{IWbemClassObject, IWbemLocator, WBEM_GENERIC_FLAG_TYPE, WbemLocator};
        use windows::core::{BSTR, PCWSTR, w};

        let _com = ComInitGuard::enter()?;
        ensure_com_security();
        let cmdline = build_quoted_command_line(exe, args);

        // SAFETY: 所有 COM 接口与 BSTR/VARIANT 参数均严格按 Windows SDK WMI Win32_Process.Create 规范传递与析构。
        unsafe {
            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("CoCreateInstance(WbemLocator): {e}"))?;
            let empty = BSTR::new();
            let services = locator
                .ConnectServer(&BSTR::from("ROOT\\CIMV2"), &empty, &empty, &empty, 0, &empty, None)
                .map_err(|e| format!("WMI ConnectServer: {e}"))?;

            CoSetProxyBlanket(
                &services,
                RPC_C_AUTHN_WINNT,
                RPC_C_AUTHZ_NONE,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )
            .map_err(|e| format!("CoSetProxyBlanket: {e}"))?;

            let class_path = BSTR::from("Win32_Process");
            let mut proc_cls: Option<IWbemClassObject> = None;
            services
                .GetObject(
                    &class_path,
                    WBEM_GENERIC_FLAG_TYPE(0),
                    None,
                    Some(&raw mut proc_cls),
                    None,
                )
                .map_err(|e| format!("WMI GetObject(Win32_Process): {e}"))?;
            let proc_cls = proc_cls.ok_or_else(|| "Win32_Process class object is null".to_string())?;

            let mut in_sig: Option<IWbemClassObject> = None;
            proc_cls
                .GetMethod(w!("Create"), 0, &raw mut in_sig, std::ptr::null_mut())
                .map_err(|e| format!("WMI GetMethod(Create): {e}"))?;
            let in_params = in_sig
                .ok_or_else(|| "Win32_Process::Create input signature is null".to_string())?
                .SpawnInstance(0)
                .map_err(|e| format!("WMI SpawnInstance: {e}"))?;

            let cmd_var = OwnedVariant::from_bstr(&cmdline);
            in_params
                .Put(w!("CommandLine"), 0, &raw const cmd_var.0, 0)
                .map_err(|e| format!("WMI Put(CommandLine): {e}"))?;

            let _dir_var = if let Some(dir) = work_dir.filter(|p| p.is_dir()) {
                let dir_var = OwnedVariant::from_bstr(&dir.to_string_lossy());
                in_params
                    .Put(w!("CurrentDirectory"), 0, &raw const dir_var.0, 0)
                    .map_err(|e| format!("WMI Put(CurrentDirectory): {e}"))?;
                Some(dir_var)
            } else {
                None
            };

            let _startup_var = if hide_window {
                build_wmi_hidden_startup_info(&services).inspect(|sv| {
                    let _ = in_params.Put(w!("ProcessStartupInformation"), 0, &raw const sv.0, 0);
                })
            } else {
                None
            };

            let mut out_params: Option<IWbemClassObject> = None;
            services
                .ExecMethod(
                    &class_path,
                    &BSTR::from("Create"),
                    WBEM_GENERIC_FLAG_TYPE(0),
                    None,
                    &in_params,
                    Some(&raw mut out_params),
                    None,
                )
                .map_err(|e| format!("WMI ExecMethod(Create): {e}"))?;
            let out_params = out_params.ok_or_else(|| "WMI ExecMethod returned null out_params".to_string())?;

            let mut ret_var = OwnedVariant::default();
            out_params
                .Get(w!("ReturnValue"), 0, &raw mut ret_var.0, None, None)
                .map_err(|e| format!("WMI Get(ReturnValue): {e}"))?;
            let ret_val = VariantToUInt32(&raw const ret_var.0).map_err(|e| format!("WMI ReturnValue decode: {e}"))?;
            if ret_val != 0 {
                return Err(format!("Win32_Process::Create failed with ReturnValue={ret_val}"));
            }

            let mut pid_var = OwnedVariant::default();
            out_params
                .Get(w!("ProcessId"), 0, &raw mut pid_var.0, None, None)
                .map_err(|e| format!("WMI Get(ProcessId): {e}"))?;
            VariantToUInt32(&raw const pid_var.0).map_err(|e| format!("WMI ProcessId decode: {e}"))
        }
    }

    /// A6 二级代理：当 WMI 服务异常时，退化走 Task Scheduler 2.0 COM (`ITaskService`) 触发脱离当前 Job 的一次性即时任务并自动删表。
    fn task_scheduler_spawn_process(exe: &Path, args: &[String], work_dir: Option<&Path>) -> Result<u32, String> {
        use windows::Win32::Foundation::{VARIANT_FALSE, VARIANT_TRUE};
        use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
        use windows::Win32::System::TaskScheduler::{
            IExecAction, ITaskService, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE, TASK_INSTANCES_PARALLEL,
            TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_LUA, TaskScheduler,
        };
        use windows::Win32::System::Variant::VARIANT;
        use windows::core::{BSTR, Interface as _};

        let _com = ComInitGuard::enter()?;
        ensure_com_security();

        let seq = super::ATOMIC_TEMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let task_name = format!(
            "ClashMiniGuard_{}_{}",
            // SAFETY: GetCurrentProcessId 无前置状态要求。
            unsafe { GetCurrentProcessId() },
            seq
        );
        let task_name_bstr = BSTR::from(task_name.as_str());
        let args_str = build_quoted_arguments(args);

        // SAFETY: 遵循 Task Scheduler 2.0 COM 接口创建、按需启动（Run）并立即清理临时任务定义。
        unsafe {
            let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("CoCreateInstance(TaskScheduler): {e}"))?;
            let empty = VARIANT::default();
            service
                .Connect(&empty, &empty, &empty, &empty)
                .map_err(|e| format!("ITaskService::Connect: {e}"))?;
            let root = service
                .GetFolder(&BSTR::from("\\"))
                .map_err(|e| format!("ITaskService::GetFolder: {e}"))?;
            let task_def = service.NewTask(0).map_err(|e| format!("ITaskService::NewTask: {e}"))?;

            let principal = task_def.Principal().map_err(|e| format!("Task::Principal: {e}"))?;
            principal
                .SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)
                .map_err(|e| format!("Principal::SetLogonType: {e}"))?;
            principal
                .SetRunLevel(TASK_RUNLEVEL_LUA)
                .map_err(|e| format!("Principal::SetRunLevel: {e}"))?;

            let settings = task_def.Settings().map_err(|e| format!("Task::Settings: {e}"))?;
            let _ = settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE);
            let _ = settings.SetStopIfGoingOnBatteries(VARIANT_FALSE);
            let _ = settings.SetExecutionTimeLimit(&BSTR::from("PT0S"));
            let _ = settings.SetHidden(VARIANT_TRUE);
            let _ = settings.SetMultipleInstances(TASK_INSTANCES_PARALLEL);

            let actions = task_def.Actions().map_err(|e| format!("Task::Actions: {e}"))?;
            let action = actions
                .Create(TASK_ACTION_EXEC)
                .map_err(|e| format!("Actions::Create: {e}"))?;
            let exec_action: IExecAction = action.cast().map_err(|e| format!("Action::cast(IExecAction): {e}"))?;
            exec_action
                .SetPath(&BSTR::from(exe.to_string_lossy().as_ref()))
                .map_err(|e| format!("ExecAction::SetPath: {e}"))?;
            if !args_str.is_empty() {
                exec_action
                    .SetArguments(&BSTR::from(args_str.as_str()))
                    .map_err(|e| format!("ExecAction::SetArguments: {e}"))?;
            }
            if let Some(dir) = work_dir.filter(|p| p.is_dir()) {
                exec_action
                    .SetWorkingDirectory(&BSTR::from(dir.to_string_lossy().as_ref()))
                    .map_err(|e| format!("ExecAction::SetWorkingDirectory: {e}"))?;
            }

            let registered = root
                .RegisterTaskDefinition(
                    &task_name_bstr,
                    &task_def,
                    TASK_CREATE_OR_UPDATE.0,
                    &empty,
                    &empty,
                    TASK_LOGON_INTERACTIVE_TOKEN,
                    &empty,
                )
                .map_err(|e| format!("RegisterTaskDefinition: {e}"))?;

            let run_res = registered.Run(&empty);
            let _ = root.DeleteTask(&task_name_bstr, 0);
            let running = run_res.map_err(|e| format!("RegisteredTask::Run: {e}"))?;
            running.EnginePID().map_err(|e| format!("RunningTask::EnginePID: {e}"))
        }
    }

    pub(super) struct SpawnedProcess {
        pub(super) pid: u32,
        owned_handle: Handle,
        child: Option<Child>,
        pub(super) broker: &'static str,
        pub(super) in_job: Option<bool>,
        pub(super) degraded: bool,
    }

    impl Drop for SpawnedProcess {
        fn drop(&mut self) {
            if !self.owned_handle.is_null() {
                // SAFETY: owned_handle 仅由本结构体独占（来自 OpenProcess），未与 Child 共享。
                unsafe {
                    let _ = CloseHandle(self.owned_handle);
                }
                self.owned_handle = std::ptr::null_mut();
            }
        }
    }

    impl SpawnedProcess {
        pub(super) fn wait_handle(&self) -> Handle {
            if let Some(ref child) = self.child {
                child.as_raw_handle() as Handle
            } else {
                self.owned_handle
            }
        }

        pub(super) fn collect_exit_code(&mut self) -> Option<u32> {
            if let Some(mut child) = self.child.take() {
                return child.wait().ok().and_then(|s| s.code()).map(|c| c as u32);
            }
            if self.owned_handle.is_null() {
                return None;
            }
            let mut code: u32 = 0;
            // SAFETY: owned_handle 拥有 PROCESS_QUERY_LIMITED_INFORMATION 权限。
            let ok = unsafe { GetExitCodeProcess(self.owned_handle, &raw mut code) };
            if ok != 0 { Some(code) } else { None }
        }
    }

    fn open_spawned_broker_process(pid: u32, broker: &'static str) -> Result<SpawnedProcess, String> {
        // SAFETY: 对刚拉起的目标 PID 申请同步与受限查询句柄，用于无轮询阻塞等待与 Job 判定。
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            // SAFETY: 紧随 OpenProcess 失败读取 GetLastError。
            let err = unsafe { GetLastError() };
            return Err(format!("OpenProcess({pid}) via {broker} failed (GetLastError={err})"));
        }
        let in_job = query_handle_in_job(handle);
        Ok(SpawnedProcess {
            pid,
            owned_handle: handle,
            child: None,
            broker,
            in_job,
            degraded: in_job != Some(false),
        })
    }

    fn spawn_via_std_command(
        exe: &Path,
        args: &[String],
        work_dir: Option<&Path>,
        flags: u32,
        broker: &'static str,
        force_degraded: bool,
    ) -> std::io::Result<SpawnedProcess> {
        let mut cmd = Command::new(exe);
        if let Some(dir) = work_dir.filter(|p| p.is_dir()) {
            cmd.current_dir(dir);
        }
        let child = cmd.args(args).creation_flags(flags).spawn()?;
        let pid = child.id();
        let in_job = query_handle_in_job(child.as_raw_handle() as Handle);
        Ok(SpawnedProcess {
            pid,
            owned_handle: std::ptr::null_mut(),
            child: Some(child),
            broker,
            in_job,
            degraded: force_degraded || in_job != Some(false),
        })
    }

    /// A6 独立进程树拉起级联：
    /// `WmiSession0` (`WmiPrvSE.exe`) -> `TaskSchedulerCom` (`ITaskService`) -> `CREATE_BREAKAWAY_FROM_JOB` -> (可选)同 Job 降级。
    pub(super) fn spawn_isolated_process_with<F>(
        exe: &Path,
        make_args: F,
        work_dir: Option<&Path>,
        allow_degraded: bool,
        hide_window: bool,
    ) -> Result<SpawnedProcess, String>
    where
        F: Fn(&str) -> Vec<String>,
    {
        let wmi_err = match wmi_spawn_process(exe, &make_args(BROKER_WMI_SESSION0), work_dir, hide_window)
            .and_then(|pid| open_spawned_broker_process(pid, BROKER_WMI_SESSION0))
        {
            Ok(proc) => return Ok(proc),
            Err(err) => err,
        };

        let ts_err = match task_scheduler_spawn_process(exe, &make_args(BROKER_TASK_SCHEDULER_COM), work_dir)
            .and_then(|pid| open_spawned_broker_process(pid, BROKER_TASK_SCHEDULER_COM))
        {
            Ok(proc) => {
                logging!(
                    warn,
                    Type::Setup,
                    "[Guardian] WMI Broker ({wmi_err}) 不可用，已切换 Task Scheduler COM 完成跨树隔离拉起"
                );
                return Ok(proc);
            }
            Err(err) => err,
        };

        let breakaway_err = match spawn_via_std_command(
            exe,
            &make_args(BROKER_BREAKAWAY_JOB),
            work_dir,
            CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB,
            BROKER_BREAKAWAY_JOB,
            false,
        ) {
            Ok(proc) => {
                logging!(
                    warn,
                    Type::Setup,
                    "[Guardian] WMI ({wmi_err}) 与 TaskScheduler ({ts_err}) 不可用，退回 Breakaway 成功 (in_job={:?})",
                    proc.in_job
                );
                return Ok(proc);
            }
            Err(err) => err,
        };

        if !allow_degraded {
            return Err(format!(
                "WMI={wmi_err}; TaskScheduler={ts_err}; Breakaway={breakaway_err}"
            ));
        }

        let proc = spawn_via_std_command(
            exe,
            &make_args(BROKER_SAME_JOB_DEGRADED),
            work_dir,
            CREATE_NO_WINDOW,
            BROKER_SAME_JOB_DEGRADED,
            true,
        )
        .map_err(|e| format!("WMI={wmi_err}; TaskScheduler={ts_err}; Breakaway={breakaway_err}; Direct={e}"))?;
        logging!(
            error,
            Type::Setup,
            "[Guardian] 所有跨树逃逸代理均失败，已降级同 Job 运行: WMI={wmi_err}; TS={ts_err}; Breakaway={breakaway_err}"
        );
        Ok(proc)
    }

    fn build_companion_args(main_exe: &Path, session: &GuardianSession, generation: u32, broker: &str) -> Vec<String> {
        vec![
            "--guardian".to_string(),
            "--parent-pid".to_string(),
            session.pid.to_string(),
            "--target-exe".to_string(),
            main_exe.to_string_lossy().into_owned(),
            "--log-dir".to_string(),
            session.log_dir.to_string_lossy().into_owned(),
            "--guard-dir".to_string(),
            session.guard_dir.to_string_lossy().into_owned(),
            "--singleton-port".to_string(),
            session.singleton_port.to_string(),
            "--gen".to_string(),
            generation.to_string(),
            "--session-id".to_string(),
            session.session_id.clone(),
            "--session-epoch".to_string(),
            session.session_epoch.clone(),
            "--broker".to_string(),
            broker.to_string(),
        ]
    }

    fn spawn_companion_child(
        guard_exe: &Path,
        main_exe: &Path,
        session: &GuardianSession,
        generation: u32,
    ) -> Result<SpawnedProcess, String> {
        let work_dir = guard_exe.parent();
        let spawned = spawn_isolated_process_with(
            guard_exe,
            |broker| build_companion_args(main_exe, session, generation, broker),
            work_dir,
            true,
            true,
        )?;
        if spawned.degraded || spawned.broker != BROKER_WMI_SESSION0 {
            append_structured_guardian_log(
                &session.log_dir,
                if spawned.degraded { "ERROR" } else { "WARN" },
                &session.session_id,
                session.pid,
                Some(spawned.pid),
                generation,
                if spawned.degraded {
                    "BREAKAWAY_DEGRADED"
                } else {
                    "SUPERVISOR_BROKER_FALLBACK"
                },
                &format!(
                    "伴生看门狗经 broker={} 拉起 (parent_in_job={:?}, guard_in_job={:?}, degraded={})",
                    spawned.broker, session.parent_in_job, spawned.in_job, spawned.degraded
                ),
            );
        }
        Ok(spawned)
    }

    fn wait_for_companion_or_exit(proc: &SpawnedProcess, exit_event: isize) -> u32 {
        let child_handle = proc.wait_handle();
        // SAFETY: child_handle 在 SpawnedProcess 存活期间有效，exit_event 为进程生命周期常驻事件句柄。
        unsafe {
            if exit_event == 0 {
                WaitForSingleObject(child_handle, INFINITE)
            } else {
                let handles = [child_handle, exit_event as Handle];
                WaitForMultipleObjects(2, handles.as_ptr(), 0, INFINITE)
            }
        }
    }

    /// A6：遭遇安装器/卸载器时在 Supervisor 内有界轮询等待（最长 120s），避免升级失败或取消后永久失防。
    fn wait_while_installer_running(session: &GuardianSession, generation: u32) -> bool {
        let wait_start = Instant::now();
        append_structured_guardian_log(
            &session.log_dir,
            "INFO",
            &session.session_id,
            session.pid,
            None,
            generation,
            "SUPERVISOR_INSTALLER_WAIT",
            "检测到安装或卸载程序运行中，暂停拉起看门狗进入有界避让等待",
        );

        while wait_start.elapsed().as_secs() < INSTALLER_WAIT_MAX_SECS {
            if wait_exit_or_timeout(session.exit_event, 5_000) || is_exit_event_signaled(session.exit_event) {
                append_structured_guardian_log(
                    &session.log_dir,
                    "INFO",
                    &session.session_id,
                    session.pid,
                    None,
                    generation,
                    "SUPERVISOR_EXIT_SIGNALED",
                    "安装器避让等待期间收到主动退出信号，监督线程退出",
                );
                return false;
            }
            if !is_installer_or_uninstaller_running() {
                append_structured_guardian_log(
                    &session.log_dir,
                    "INFO",
                    &session.session_id,
                    session.pid,
                    None,
                    generation,
                    "SUPERVISOR_INSTALLER_CLEARED",
                    "安装器已退出且主进程仍在存活，恢复伴生看门狗守护",
                );
                return true;
            }
        }

        append_structured_guardian_log(
            &session.log_dir,
            "WARN",
            &session.session_id,
            session.pid,
            None,
            generation,
            "SUPERVISOR_INSTALLER_WAIT_TIMEOUT",
            &format!(
                "安装器避让等待已超 {}s 阈值，监督线程暂时退场（若主进程仍在后续将由后台巡检重新武装）",
                INSTALLER_WAIT_MAX_SECS
            ),
        );
        false
    }

    fn run_companion_supervisor(session: &'static GuardianSession, main_exe: PathBuf) {
        // RAII 保证任何正常、异常、超限、panic 路径退出时都会重置运行标记，杜绝假阳性锁死（A6）
        let _running_guard = SupervisorRunningGuard;
        let mut rebuild_count: u32 = 0;
        let mut rapid_deaths: u32 = 0;

        loop {
            if is_exit_event_signaled(session.exit_event)
                || crate::core::handle::Handle::global().is_exiting()
                || is_system_shutting_down()
            {
                append_structured_guardian_log(
                    &session.log_dir,
                    "INFO",
                    &session.session_id,
                    session.pid,
                    None,
                    session.initial_gen,
                    "SUPERVISOR_EXIT_SIGNALED",
                    "检测到主动退出事件或系统关机，监督线程正常退出",
                );
                break;
            }

            if is_installer_or_uninstaller_running() && !wait_while_installer_running(session, session.initial_gen) {
                break;
            }

            let state = read_respawn_state_from(&session.guard_dir);
            let effective_gen = session.initial_gen.max(state.respawn_count);
            let guard_exe = resolve_guardian_executable(&main_exe, &session.guard_dir, session, effective_gen);

            let mut child = match spawn_companion_child(&guard_exe, &main_exe, session, effective_gen) {
                Ok(proc) => proc,
                Err(err) => {
                    rapid_deaths = rapid_deaths.saturating_add(1);
                    let retry_ms = compute_guard_rebuild_backoff_ms(rapid_deaths);
                    logging!(
                        warn,
                        Type::Setup,
                        "[Guardian] 启动伴生看门狗失败 ({})，将在 {}ms 后重试",
                        err,
                        retry_ms
                    );
                    append_structured_guardian_log(
                        &session.log_dir,
                        "WARN",
                        &session.session_id,
                        session.pid,
                        None,
                        effective_gen,
                        "SUPERVISOR_SPAWN_RETRY",
                        &format!("拉起看门狗子进程失败，{retry_ms}ms 后重试 (rapid_deaths={rapid_deaths}): {err}"),
                    );
                    if rapid_deaths >= 5 {
                        append_structured_guardian_log(
                            &session.log_dir,
                            "ERROR",
                            &session.session_id,
                            session.pid,
                            None,
                            effective_gen,
                            "SUPERVISOR_SPAWN_FAILED_LIMIT",
                            "连续拉起看门狗失败达 5 次上限，监督线程退出并释放重武装锁",
                        );
                        break;
                    }
                    if wait_exit_or_timeout(session.exit_event, retry_ms) {
                        append_structured_guardian_log(
                            &session.log_dir,
                            "INFO",
                            &session.session_id,
                            session.pid,
                            None,
                            effective_gen,
                            "SUPERVISOR_EXIT_SIGNALED",
                            "看门狗重试退避期间收到退出信号，监督线程退出",
                        );
                        break;
                    }
                    continue;
                }
            };

            let guard_pid = child.pid;
            let degraded = child.degraded;
            let broker = child.broker;
            let guard_in_job = child.in_job;
            session.current_guard_pid.store(guard_pid, Ordering::Release);
            session.breakaway_degraded.store(degraded, Ordering::Release);
            session
                .guard_in_job
                .store(guard_in_job.map_or(-1, i8::from), Ordering::Release);
            if let Ok(mut slot) = session.spawn_broker.lock() {
                *slot = Some(broker.to_string());
            }
            write_main_heartbeat_snapshot(session, current_unix_ms());

            let guard_spawned_at = Instant::now();
            if rebuild_count == 0 {
                logging!(
                    info,
                    Type::Setup,
                    "[Guardian] 伴生看门狗已就绪 (guardian_pid={}, parent_pid={}, gen={}, parent_in_job={:?}, guard_in_job={:?}, broker={}, breakaway_degraded={}, guard_exe={})",
                    guard_pid,
                    session.pid,
                    effective_gen,
                    session.parent_in_job,
                    guard_in_job,
                    broker,
                    degraded,
                    guard_exe.display()
                );
                append_structured_guardian_log(
                    &session.log_dir,
                    "INFO",
                    &session.session_id,
                    session.pid,
                    Some(guard_pid),
                    effective_gen,
                    "SUPERVISOR_SPAWN_OK",
                    &format!(
                        "伴生看门狗已就绪 (broker={broker}, parent_in_job={:?}, guard_in_job={guard_in_job:?}, breakaway_degraded={degraded}, exe={})",
                        session.parent_in_job,
                        guard_exe.display()
                    ),
                );
            } else {
                logging!(
                    warn,
                    Type::System,
                    "[Guardian] 伴生看门狗已自动重建恢复 (guardian_pid={}, parent_pid={}, broker={}, 第{}次重建)",
                    guard_pid,
                    session.pid,
                    broker,
                    rebuild_count
                );
                append_structured_guardian_log(
                    &session.log_dir,
                    "WARN",
                    &session.session_id,
                    session.pid,
                    Some(guard_pid),
                    effective_gen,
                    "SUPERVISOR_REBUILD_OK",
                    &format!(
                        "伴生看门狗第 {rebuild_count} 次重建成功 (broker={broker}, guard_in_job={guard_in_job:?}, breakaway_degraded={degraded})"
                    ),
                );
            }

            let wait_res = wait_for_companion_or_exit(&child, session.exit_event);
            session.current_guard_pid.store(0, Ordering::Release);

            if is_exit_event_signaled(session.exit_event)
                || crate::core::handle::Handle::global().is_exiting()
                || is_system_shutting_down()
            {
                append_structured_guardian_log(
                    &session.log_dir,
                    "INFO",
                    &session.session_id,
                    session.pid,
                    Some(guard_pid),
                    effective_gen,
                    "SUPERVISOR_EXIT_SIGNALED",
                    "正常退出路径唤醒，释放子进程句柄立即退场",
                );
                break;
            }

            if wait_res == WAIT_FAILED {
                // SAFETY: WaitForMultipleObjects/WaitForSingleObject 失败后紧随读取 GetLastError。
                let err = unsafe { GetLastError() };
                append_structured_guardian_log(
                    &session.log_dir,
                    "ERROR",
                    &session.session_id,
                    session.pid,
                    Some(guard_pid),
                    effective_gen,
                    "SUPERVISOR_WAIT_ERROR",
                    &format!("等待看门狗子进程内核句柄失败 (GetLastError={err})，监督线程退场等待巡检重武装"),
                );
                break;
            }

            if wait_res != WAIT_OBJECT_0 {
                append_structured_guardian_log(
                    &session.log_dir,
                    "INFO",
                    &session.session_id,
                    session.pid,
                    Some(guard_pid),
                    effective_gen,
                    "SUPERVISOR_WAIT_NON_CHILD",
                    &format!("等待看门狗返回非子进程对象 (wait_res={wait_res})，退出监督"),
                );
                break;
            }

            let exit_code = child.collect_exit_code().unwrap_or(0);
            let guard_uptime = guard_spawned_at.elapsed().as_secs();
            if guard_uptime >= GUARD_RAPID_DEATH_WINDOW_SECS {
                rapid_deaths = 0;
            } else {
                rapid_deaths = rapid_deaths.saturating_add(1);
            }
            let backoff_ms = compute_guard_rebuild_backoff_ms(rapid_deaths);

            logging!(
                warn,
                Type::System,
                "[Guardian] 检测到伴生看门狗进程意外终止 (guardian_pid={}, 存活={}s, ExitCode={})，将在 {}ms 后自动重建守护",
                guard_pid,
                guard_uptime,
                format_exit_code_diagnosis(exit_code),
                backoff_ms
            );
            append_structured_guardian_log(
                &session.log_dir,
                "WARN",
                &session.session_id,
                session.pid,
                Some(guard_pid),
                effective_gen,
                "SUPERVISOR_GUARD_DIED",
                &format!(
                    "看门狗意外终止 (uptime={}s, ExitCode={})，{}ms 后尝试重建",
                    guard_uptime,
                    format_exit_code_diagnosis(exit_code),
                    backoff_ms
                ),
            );

            if wait_exit_or_timeout(session.exit_event, backoff_ms) {
                append_structured_guardian_log(
                    &session.log_dir,
                    "INFO",
                    &session.session_id,
                    session.pid,
                    None,
                    effective_gen,
                    "SUPERVISOR_EXIT_SIGNALED",
                    "看门狗重建退避期收到退出信号，监督线程退出",
                );
                break;
            }
            rebuild_count = rebuild_count.saturating_add(1);
        }
    }

    fn spawn_supervisor_thread(session: &'static GuardianSession) {
        if GUARDIAN_SUPERVISOR_STARTED.swap(true, Ordering::AcqRel) {
            return;
        }
        let Ok(main_exe) = std::env::current_exe() else {
            GUARDIAN_SUPERVISOR_STARTED.store(false, Ordering::Release);
            append_structured_guardian_log(
                &session.log_dir,
                "ERROR",
                &session.session_id,
                session.pid,
                None,
                session.initial_gen,
                "SUPERVISOR_NO_MAIN_EXE",
                "获取 current_exe 失败，无法启动监督线程",
            );
            return;
        };

        if let Err(err) = std::thread::Builder::new()
            .name("clash-mini-guard-sup".to_string())
            .spawn(move || {
                run_companion_supervisor(session, main_exe);
            })
        {
            GUARDIAN_SUPERVISOR_STARTED.store(false, Ordering::Release);
            logging!(warn, Type::Setup, "[Guardian] 创建伴生看门狗监督线程失败: {}", err);
            append_structured_guardian_log(
                &session.log_dir,
                "ERROR",
                &session.session_id,
                session.pid,
                None,
                session.initial_gen,
                "SUPERVISOR_THREAD_SPAWN_FAILED",
                &format!("创建监督线程失败: {err}"),
            );
        }
    }

    pub(super) fn init_session_and_spawn_companion(
        log_dir: &Path,
        singleton_port: u16,
        current_gen: u32,
        inherited_epoch: Option<&str>,
    ) {
        let pid = unsafe { GetCurrentProcessId() };
        let session = GUARDIAN_SESSION.get_or_init(|| {
            let guard_dir = resolve_isolated_guardian_dir();
            let session_id = format!("s-{pid}-{}", current_unix_ms());
            let parent_in_job = query_current_process_in_job();
            let exit_name = to_wide_null(&exit_event_name(pid));
            let lw_name = to_wide_null(&lightweight_event_name(pid));
            let initial_lw = i32::from(crate::module::lightweight::is_in_lightweight_mode());
            // SAFETY: 创建当前会话专属的手动重置命名事件句柄，随进程全生命周期保留。
            let (exit_ev, exit_err, lw_ev, lw_err) = unsafe {
                let exit_ev = CreateEventW(std::ptr::null(), 1, 0, exit_name.as_ptr());
                let exit_err = if exit_ev.is_null() { GetLastError() } else { 0 };
                let lw_ev = CreateEventW(std::ptr::null(), 1, initial_lw, lw_name.as_ptr());
                let lw_err = if lw_ev.is_null() { GetLastError() } else { 0 };
                (exit_ev, exit_err, lw_ev, lw_err)
            };

            if exit_ev.is_null() {
                logging!(
                    warn,
                    Type::Setup,
                    "[Guardian] CreateEventW(exit_event) 失败: {}",
                    exit_err
                );
                append_structured_guardian_log(
                    log_dir,
                    "ERROR",
                    &session_id,
                    pid,
                    None,
                    current_gen,
                    "CREATE_EXIT_EVENT_FAILED",
                    &format!("CreateEventW(exit_event) 失败 (GetLastError={exit_err})"),
                );
            }
            if lw_ev.is_null() {
                logging!(
                    warn,
                    Type::Setup,
                    "[Guardian] CreateEventW(lightweight_event) 失败: {}",
                    lw_err
                );
                append_structured_guardian_log(
                    log_dir,
                    "ERROR",
                    &session_id,
                    pid,
                    None,
                    current_gen,
                    "CREATE_LW_EVENT_FAILED",
                    &format!("CreateEventW(lightweight_event) 失败 (GetLastError={lw_err})"),
                );
            }

            let boot_fp = current_boot_fingerprint();
            let had_stale_proof = guard_dir.join(INTENTIONAL_EXIT_FILENAME).exists();
            clear_intentional_exit_proof_in(&guard_dir);
            if had_stale_proof {
                append_structured_guardian_log(
                    log_dir,
                    "INFO",
                    &session_id,
                    pid,
                    None,
                    current_gen,
                    "INTENTIONAL_EXIT_PROOF_CLEARED_ON_START",
                    "启动时已清理残留主动退出凭证文件",
                );
            }
            let session_epoch = if is_guardian_respawn() {
                inherited_epoch
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        let st = read_respawn_state_from(&guard_dir);
                        (!st.session_epoch_id.is_empty()).then_some(st.session_epoch_id)
                    })
                    .unwrap_or_else(|| session_id.clone())
            } else {
                reset_respawn_state_for_clean_start_in(&guard_dir, Some(log_dir), &session_id, &boot_fp);
                session_id.clone()
            };

            GuardianSession {
                pid,
                session_id,
                session_epoch,
                log_dir: log_dir.to_path_buf(),
                guard_dir,
                singleton_port,
                initial_gen: current_gen,
                exit_event: exit_ev as isize,
                lightweight_event: lw_ev as isize,
                parent_in_job,
                breakaway_degraded: AtomicBool::new(false),
                guard_in_job: AtomicI8::new(-1),
                spawn_broker: Mutex::new(None),
                current_guard_pid: AtomicU32::new(0),
                last_heartbeat_unix_ms: AtomicU64::new(0),
            }
        });

        let in_lw = crate::module::lightweight::is_in_lightweight_mode();
        sync_lightweight_event_and_log(session, in_lw, "session_init");
        write_main_heartbeat_snapshot(session, current_unix_ms());
        spawn_supervisor_thread(session);
    }

    fn sync_lightweight_event_and_log(session: &GuardianSession, in_lightweight: bool, stage: &str) {
        if session.lightweight_event == 0 {
            return;
        }
        if let Err(err) = signal_event_with_retry(session.lightweight_event, in_lightweight) {
            logging!(
                warn,
                Type::System,
                "[Guardian] 同步轻量信号内核事件失败 (stage={}, target={}, GetLastError={})",
                stage,
                in_lightweight,
                err
            );
            append_structured_guardian_log(
                &session.log_dir,
                "WARN",
                &session.session_id,
                session.pid,
                None,
                session.initial_gen,
                "SET_LIGHTWEIGHT_EVENT_FAILED",
                &format!("设置或回读 lightweight_event={in_lightweight} 失败 (stage={stage}, err={err})"),
            );
        } else {
            append_structured_guardian_log(
                &session.log_dir,
                "INFO",
                &session.session_id,
                session.pid,
                None,
                session.initial_gen,
                "LIGHTWEIGHT_SIGNAL_SYNCED",
                &format!("同步轻量信号内核事件成功 (stage={stage}, target={in_lightweight})"),
            );
            write_main_heartbeat_snapshot(session, current_unix_ms());
        }
    }

    pub(super) fn ensure_guardian_supervisor() {
        let Some(session) = GUARDIAN_SESSION.get() else {
            super::start_guardian_companion();
            return;
        };
        if GUARDIAN_SUPERVISOR_STARTED.load(Ordering::Acquire)
            || is_exit_event_signaled(session.exit_event)
            || crate::core::handle::Handle::global().is_exiting()
            || is_system_shutting_down()
            || is_installer_or_uninstaller_running()
        {
            return;
        }
        append_structured_guardian_log(
            &session.log_dir,
            "WARN",
            &session.session_id,
            session.pid,
            None,
            session.initial_gen,
            "SUPERVISOR_REARMED",
            "后台巡检发现看门狗监督线程未激活，正在自动重新武装",
        );
        spawn_supervisor_thread(session);
    }

    pub(super) fn set_lightweight_mode_signal(in_lightweight: bool) {
        let Some(session) = GUARDIAN_SESSION.get() else {
            return;
        };
        sync_lightweight_event_and_log(session, in_lightweight, "mode_toggle");
    }

    pub(super) fn write_intentional_exit_phase(phase: &str, reason: &str) {
        let now_ms = current_unix_ms();
        let (pid, session_id, log_dir, guard_dir, exit_event) = if let Some(session) = GUARDIAN_SESSION.get() {
            (
                session.pid,
                session.session_id.clone(),
                Some(session.log_dir.clone()),
                session.guard_dir.clone(),
                session.exit_event,
            )
        } else {
            let pid = unsafe { GetCurrentProcessId() };
            (
                pid,
                format!("early-{pid}"),
                crate::utils::dirs::app_logs_dir().ok(),
                resolve_isolated_guardian_dir(),
                0,
            )
        };

        let proof = IntentionalExitProof {
            pid,
            session_id: session_id.clone(),
            phase: phase.to_string(),
            timestamp_unix_ms: now_ms,
            timestamp_local: format_local_timestamp(),
            reason: reason.to_string(),
        };
        if let Err(err) = write_intentional_exit_proof_with_retry(&guard_dir, &proof) {
            logging!(
                warn,
                Type::System,
                "[Guardian] 写入主动退出持久化凭证失败 (phase={}, reason={}): {}",
                phase,
                reason,
                err
            );
            if let Some(ref dir) = log_dir {
                append_structured_guardian_log(
                    dir,
                    "WARN",
                    &session_id,
                    pid,
                    None,
                    0,
                    "INTENTIONAL_EXIT_PROOF_WRITE_FAILED",
                    &format!("写入主动退出持久化凭证失败 (phase={phase}, reason={reason}): {err}"),
                );
            }
        }

        if exit_event != 0 {
            if let Err(err) = signal_event_with_retry(exit_event, true) {
                if let Some(ref dir) = log_dir {
                    append_structured_guardian_log(
                        dir,
                        "ERROR",
                        &session_id,
                        pid,
                        None,
                        0,
                        "SET_EXIT_EVENT_FAILED",
                        &format!(
                            "置位 exit_event 失败 (phase={phase}, reason={reason}, err={err})，已落盘带外退出凭证兜底"
                        ),
                    );
                }
            } else if let Some(ref dir) = log_dir {
                append_structured_guardian_log(
                    dir,
                    "INFO",
                    &session_id,
                    pid,
                    None,
                    0,
                    "INTENTIONAL_EXIT_SIGNALED",
                    &format!("已双写主动退出凭证与内核事件 (phase={phase}, reason={reason})"),
                );
            }
        }
    }

    pub(super) fn cancel_intentional_exit(reason: &str) {
        let Some(session) = GUARDIAN_SESSION.get() else {
            let guard_dir = resolve_isolated_guardian_dir();
            let had_proof = guard_dir.join(INTENTIONAL_EXIT_FILENAME).exists();
            clear_intentional_exit_proof_in(&guard_dir);
            if had_proof && let Ok(log_dir) = crate::utils::dirs::app_logs_dir() {
                let pid = unsafe { GetCurrentProcessId() };
                append_structured_guardian_log(
                    &log_dir,
                    "INFO",
                    &format!("early-{pid}"),
                    pid,
                    None,
                    0,
                    "INTENTIONAL_EXIT_CANCELED_EARLY",
                    &format!("ExitRequested 在会话初始化前被阻止，已清理主动退出凭证 (reason={reason})"),
                );
            }
            return;
        };
        clear_intentional_exit_proof_in(&session.guard_dir);
        if let Err(err) = signal_event_with_retry(session.exit_event, false) {
            append_structured_guardian_log(
                &session.log_dir,
                "WARN",
                &session.session_id,
                session.pid,
                None,
                session.initial_gen,
                "RESET_EXIT_EVENT_FAILED",
                &format!("撤销 exit_event 失败 (reason={reason}, err={err})"),
            );
        } else {
            append_structured_guardian_log(
                &session.log_dir,
                "INFO",
                &session.session_id,
                session.pid,
                None,
                session.initial_gen,
                "INTENTIONAL_EXIT_CANCELED",
                &format!("ExitRequested 被阻止，已同步撤销退出凭证与内核事件 (reason={reason})"),
            );
        }
        ensure_guardian_supervisor();
    }

    pub(super) fn get_diagnostic_summary() -> String {
        let Some(session) = GUARDIAN_SESSION.get() else {
            return format!(
                "未初始化 (guard_dir={}, job={:?})",
                resolve_isolated_guardian_dir().display(),
                query_current_process_in_job()
            );
        };
        let state = read_respawn_state_from(&session.guard_dir);
        let guard_pid = session.current_guard_pid.load(Ordering::Acquire);
        let supervisor_alive = GUARDIAN_SUPERVISOR_STARTED.load(Ordering::Acquire);
        let degraded = session.breakaway_degraded.load(Ordering::Acquire);
        let guard_in_job = match session.guard_in_job.load(Ordering::Acquire) {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };
        let broker = session
            .spawn_broker
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| "none".to_string());
        format!(
            "session={}, epoch={}, supervisor={}, guard_pid={}, broker={}, parent_in_job={:?}, guard_in_job={:?}, breakaway_degraded={}, respawn_count={}/{}, guard_dir={}",
            session.session_id,
            session.session_epoch,
            supervisor_alive,
            guard_pid,
            broker,
            session.parent_in_job,
            guard_in_job,
            degraded,
            state.respawn_count,
            MAX_RAPID_RESPAWN_GEN,
            session.guard_dir.display()
        )
    }

    fn query_process_uptime_secs(h_parent: Handle) -> u64 {
        let mut creation = FileTime::default();
        let mut exit = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();
        // SAFETY: h_parent 具备 PROCESS_QUERY_LIMITED_INFORMATION 权限。
        let ok = unsafe {
            GetProcessTimes(
                h_parent,
                &raw mut creation,
                &raw mut exit,
                &raw mut kernel,
                &raw mut user,
            )
        };
        if ok == 0 {
            return 0;
        }
        let start_100ns = creation.to_u64();
        let end_100ns = exit.to_u64();
        if end_100ns > start_100ns {
            (end_100ns - start_100ns) / 10_000_000
        } else {
            0
        }
    }

    fn is_system_shutting_down() -> bool {
        // SAFETY: GetSystemMetrics 无前置状态要求。
        unsafe { GetSystemMetrics(SM_SHUTTINGDOWN) != 0 }
    }

    fn pump_pending_window_messages() {
        let mut msg = Msg::default();
        // SAFETY: 排空当前守护线程哨兵窗口的消息队列，使 IsHungAppWindow 探测恒返回 FALSE。
        unsafe {
            while PeekMessageW(&raw mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                let _ = TranslateMessage(&raw const msg);
                let _ = DispatchMessageW(&raw const msg);
            }
        }
    }

    fn record_guard_heartbeat(
        guard_dir: &Path,
        args: &GuardianArgs,
        guard_pid: u32,
        session_id: &str,
        guard_in_job: Option<bool>,
        status: &str,
    ) {
        let now_ms = current_unix_ms();
        let hb = GuardHeartbeat {
            pid: guard_pid,
            parent_pid: args.parent_pid,
            session_id: session_id.to_string(),
            generation: args.generation,
            guard_in_job,
            spawn_broker: args.broker.clone(),
            status: status.to_string(),
            last_seen_unix_ms: now_ms,
            last_seen_local: format_local_timestamp(),
        };
        let _ = write_guard_heartbeat_to(guard_dir, &hb);
    }

    /// 阻塞等待父进程退出或主动退出事件，同时每 15s 刷新诊断心跳并响应哨兵窗口消息（平时 0% CPU 内核态挂起）。
    fn wait_parent_with_message_pump(
        h_parent: Handle,
        h_exit: Handle,
        args: &GuardianArgs,
        guard_dir: &Path,
        guard_pid: u32,
        session_id: &str,
        guard_in_job: Option<bool>,
    ) -> u32 {
        let handles = [h_parent, h_exit];
        let count: u32 = if h_exit.is_null() { 1 } else { 2 };
        let timeout_ms = HEARTBEAT_INTERVAL_MS.min(u64::from(INFINITE - 1)) as u32;
        loop {
            // SAFETY: handles 前 count 项均为有效内核同步句柄。
            let res = unsafe { MsgWaitForMultipleObjects(count, handles.as_ptr(), 0, timeout_ms, QS_ALLINPUT) };
            if res == WAIT_OBJECT_0 + count {
                pump_pending_window_messages();
            } else if res == WAIT_TIMEOUT {
                pump_pending_window_messages();
                record_guard_heartbeat(guard_dir, args, guard_pid, session_id, guard_in_job, "watching");
            } else {
                return res;
            }
        }
    }

    fn check_intentional_exit_proof_for_guardian(
        guard_dir: &Path,
        args: &GuardianArgs,
        guard_pid: u32,
        session_id: &str,
    ) -> Option<String> {
        match evaluate_intentional_exit_proof(guard_dir, args.parent_pid, session_id, current_unix_ms()) {
            IntentionalProofCheck::Valid { phase, reason } => Some(format!("{phase}:{reason}")),
            IntentionalProofCheck::NotPresent => None,
            IntentionalProofCheck::Rejected { detail } => {
                append_structured_guardian_log(
                    &args.log_dir,
                    "WARN",
                    session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    "GUARD_INTENTIONAL_PROOF_REJECTED",
                    &detail,
                );
                None
            }
        }
    }

    pub(super) fn run_guardian_loop(args: &GuardianArgs) {
        let guard_dir = args.resolved_guard_dir();
        let session_id = args.resolved_session_id();
        // SAFETY: 当前在 `--guardian` 独立子进程中获取当前 PID 无副作用。
        let guard_pid = unsafe { GetCurrentProcessId() };
        let guard_in_job = query_current_process_in_job();

        let exit_name = to_wide_null(&exit_event_name(args.parent_pid));
        let lw_name = to_wide_null(&lightweight_event_name(args.parent_pid));

        // SAFETY: 打开父进程同步句柄与命名事件句柄，挂载自身哨兵窗口，退出前完整销毁并 CloseHandle。
        unsafe {
            let h_parent = OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                args.parent_pid,
            );
            if h_parent.is_null() {
                let err = GetLastError();
                append_structured_guardian_log(
                    &args.log_dir,
                    "ERROR",
                    &session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    "GUARD_OPEN_PARENT_FAILED",
                    &format!("OpenProcess({}) 失败 (GetLastError={err})，看门狗退出", args.parent_pid),
                );
                return;
            }

            let guard_sentinel = create_sentinel_window_pair("ClashMiniGuardHost", "Clash Mini Guard Host");
            pump_pending_window_messages();

            let h_exit = OpenEventW(SYNCHRONIZE | EVENT_MODIFY_STATE, 0, exit_name.as_ptr());
            let h_exit_err = if h_exit.is_null() { GetLastError() } else { 0 };
            let h_lw = OpenEventW(SYNCHRONIZE, 0, lw_name.as_ptr());
            let h_lw_err = if h_lw.is_null() { GetLastError() } else { 0 };

            if h_exit.is_null() || h_lw.is_null() {
                append_structured_guardian_log(
                    &args.log_dir,
                    "WARN",
                    &session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    "GUARD_EVENT_OPEN_FAILED",
                    &format!(
                        "打开宿主内核事件存在异常 (h_exit_err={h_exit_err}, h_lw_err={h_lw_err})，将依赖带外持久化凭证兜底"
                    ),
                );
            }

            append_structured_guardian_log(
                &args.log_dir,
                "INFO",
                &session_id,
                args.parent_pid,
                Some(guard_pid),
                args.generation,
                "GUARD_STARTED",
                &format!(
                    "看门狗已进入守护等待环 (broker={:?}, guard_in_job={guard_in_job:?})",
                    args.broker
                ),
            );
            record_guard_heartbeat(&guard_dir, args, guard_pid, &session_id, guard_in_job, "watching");

            let wait_result =
                wait_parent_with_message_pump(h_parent, h_exit, args, &guard_dir, guard_pid, &session_id, guard_in_job);
            destroy_sentinel_window_pair(guard_sentinel);

            let wait_err = if wait_result == WAIT_FAILED { GetLastError() } else { 0 };

            // 若正常退出事件已触发，或存在有效的带外主动退出凭证，或系统正在关机，0ms 静默退场
            let intentional_by_event = !h_exit.is_null()
                && (wait_result == WAIT_OBJECT_0 + 1 || WaitForSingleObject(h_exit, 0) == WAIT_OBJECT_0);
            let intentional_reason =
                check_intentional_exit_proof_for_guardian(&guard_dir, args, guard_pid, &session_id);

            if intentional_by_event || intentional_reason.is_some() || is_system_shutting_down() {
                let code = if is_system_shutting_down() {
                    "GUARD_SYSTEM_SHUTDOWN"
                } else if intentional_by_event {
                    "GUARD_INTENTIONAL_EXIT_EVENT"
                } else {
                    "GUARD_INTENTIONAL_EXIT_PROOF"
                };
                append_structured_guardian_log(
                    &args.log_dir,
                    "INFO",
                    &session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    code,
                    &format!(
                        "收到主动退出或关机信号，看门狗静默退场 (event={intentional_by_event}, proof={intentional_reason:?})"
                    ),
                );
                record_guard_heartbeat(
                    &guard_dir,
                    args,
                    guard_pid,
                    &session_id,
                    guard_in_job,
                    "exiting_intentional",
                );
                if !h_lw.is_null() {
                    let _ = CloseHandle(h_lw);
                }
                if !h_exit.is_null() {
                    let _ = CloseHandle(h_exit);
                }
                let _ = CloseHandle(h_parent);
                return;
            }

            if wait_result != WAIT_OBJECT_0 {
                append_structured_guardian_log(
                    &args.log_dir,
                    "ERROR",
                    &session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    "GUARD_WAIT_FAILED",
                    &format!("MsgWaitForMultipleObjects 返回异常 (wait_result={wait_result}, GetLastError={wait_err})"),
                );
                if !h_lw.is_null() {
                    let _ = CloseHandle(h_lw);
                }
                if !h_exit.is_null() {
                    let _ = CloseHandle(h_exit);
                }
                let _ = CloseHandle(h_parent);
                return;
            }

            let mut raw_exit_code: u32 = 0;
            let exit_code = if GetExitCodeProcess(h_parent, &raw mut raw_exit_code) != 0 {
                Some(raw_exit_code)
            } else {
                let err = GetLastError();
                append_structured_guardian_log(
                    &args.log_dir,
                    "WARN",
                    &session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    "GUARD_GET_EXIT_CODE_FAILED",
                    &format!("GetExitCodeProcess 失败 (GetLastError={err})，退出码标记为未知"),
                );
                None
            };
            let uptime_secs = query_process_uptime_secs(h_parent);
            let in_lightweight = !h_lw.is_null() && WaitForSingleObject(h_lw, 0) == WAIT_OBJECT_0;

            if !h_lw.is_null() {
                let _ = CloseHandle(h_lw);
            }
            if !h_exit.is_null() {
                let _ = CloseHandle(h_exit);
            }
            let _ = CloseHandle(h_parent);

            record_guard_heartbeat(&guard_dir, args, guard_pid, &session_id, guard_in_job, "parent_died");
            handle_unexpected_termination(
                args,
                &guard_dir,
                guard_pid,
                &session_id,
                exit_code,
                uptime_secs,
                in_lightweight,
            );
        }
    }

    fn resolve_respawn_budget_or_log(
        args: &GuardianArgs,
        guard_dir: &Path,
        guard_pid: u32,
        session_id: &str,
        boot_fp: &str,
        commit: bool,
    ) -> Option<u32> {
        let session_epoch = args.resolved_session_epoch();
        match evaluate_and_record_respawn_in(
            guard_dir,
            &session_epoch,
            args.generation,
            args.parent_pid,
            current_unix_ms(),
            boot_fp,
            commit,
        ) {
            Ok(Some(next_gen)) => Some(next_gen),
            Ok(None) => {
                append_structured_guardian_log(
                    &args.log_dir,
                    "ERROR",
                    session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    "GUARD_RESPAWN_CIRCUIT_OPEN",
                    &format!(
                        "同一开机与会话纪元内连续异常终止已达 {MAX_RAPID_RESPAWN_GEN} 次上限（不依时间窗重置，commit={commit}），熔断停止自愈拉起"
                    ),
                );
                None
            }
            Err(err) => {
                append_structured_guardian_log(
                    &args.log_dir,
                    "ERROR",
                    session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    args.generation,
                    if commit {
                        "GUARD_RESPAWN_STATE_PERSIST_FAILED"
                    } else {
                        "GUARD_RESPAWN_STATE_ERROR"
                    },
                    &format!("自愈状态文件校验/写入失败，Fail-Closed 拒绝拉起 (commit={commit}): {err}"),
                );
                None
            }
        }
    }

    fn handle_unexpected_termination(
        args: &GuardianArgs,
        guard_dir: &Path,
        guard_pid: u32,
        session_id: &str,
        exit_code: Option<u32>,
        uptime_secs: u64,
        in_lightweight: bool,
    ) {
        let diag = exit_code.map_or_else(
            || "未知 (GetExitCodeProcess 失败)".to_string(),
            format_exit_code_diagnosis,
        );
        append_structured_guardian_log(
            &args.log_dir,
            "ERROR",
            session_id,
            args.parent_pid,
            Some(guard_pid),
            args.generation,
            "GUARD_PARENT_DIED",
            &format!(
                "检测到主进程非正常终止 (存活={}s, in_lightweight={}, ExitCode={})",
                uptime_secs, in_lightweight, diag
            ),
        );

        let boot_fp = current_boot_fingerprint();
        if resolve_respawn_budget_or_log(args, guard_dir, guard_pid, session_id, &boot_fp, false).is_none() {
            return;
        }

        append_structured_guardian_log(
            &args.log_dir,
            "INFO",
            session_id,
            args.parent_pid,
            Some(guard_pid),
            args.generation,
            "GUARD_SAFETY_DELAY_ENTER",
            &format!("进入 {RESPAWN_SAFETY_DELAY_MS}ms 安全避让窗口（等待安装/卸载器与主动退出凭证最终收敛）"),
        );
        // 安全避让窗口：给安装/卸载器起手以及带外退出凭证最终落盘留足竞争余量（A7）
        std::thread::sleep(Duration::from_millis(RESPAWN_SAFETY_DELAY_MS));

        if is_system_shutting_down() {
            append_structured_guardian_log(
                &args.log_dir,
                "INFO",
                session_id,
                args.parent_pid,
                Some(guard_pid),
                args.generation,
                "GUARD_SYSTEM_SHUTDOWN",
                "避让窗口期检测到系统正在关机，放弃自愈拉起",
            );
            return;
        }

        if let Some(reason) = check_intentional_exit_proof_for_guardian(guard_dir, args, guard_pid, session_id) {
            append_structured_guardian_log(
                &args.log_dir,
                "INFO",
                session_id,
                args.parent_pid,
                Some(guard_pid),
                args.generation,
                "GUARD_INTENTIONAL_EXIT_PROOF",
                &format!("避让复核命中带外主动退出凭证 (proof={reason})，放弃拉起"),
            );
            return;
        }

        if !args.target_exe.exists() {
            append_structured_guardian_log(
                &args.log_dir,
                "INFO",
                session_id,
                args.parent_pid,
                Some(guard_pid),
                args.generation,
                "GUARD_SKIP_TARGET_MISSING",
                &format!("目标主程序不存在 ({})，看门狗主动退出", args.target_exe.display()),
            );
            return;
        }

        if is_installer_or_uninstaller_running() {
            append_structured_guardian_log(
                &args.log_dir,
                "INFO",
                session_id,
                args.parent_pid,
                Some(guard_pid),
                args.generation,
                "GUARD_SKIP_INSTALLER_RUNNING",
                "检测到安装或卸载程序正在运行，看门狗主动避让退出",
            );
            return;
        }

        // 若单例端口已被用户手动重启的新实例绑定，则主动避让
        if TcpListener::bind(("127.0.0.1", args.singleton_port)).is_err() {
            append_structured_guardian_log(
                &args.log_dir,
                "INFO",
                session_id,
                args.parent_pid,
                Some(guard_pid),
                args.generation,
                "GUARD_SKIP_PORT_BOUND",
                &format!("单例端口 {} 已被新实例占用，看门狗主动避让退出", args.singleton_port),
            );
            return;
        }

        let Some(next_gen) = resolve_respawn_budget_or_log(args, guard_dir, guard_pid, session_id, &boot_fp, true)
        else {
            return;
        };

        let session_epoch = args.resolved_session_epoch();
        let mut launch_args = vec![
            "--guardian-respawn".to_string(),
            "--guardian-gen".to_string(),
            next_gen.to_string(),
            "--session-epoch".to_string(),
            session_epoch,
            "--prev-pid".to_string(),
            args.parent_pid.to_string(),
        ];
        if let Some(code) = exit_code {
            launch_args.push("--prev-exit".to_string());
            launch_args.push(code.to_string());
        }
        // A5：不再一刀切丢弃非轻量态保护 —— 轻量态补 `--silent` 恢复托盘，非轻量态恢复可见 GUI（均携带 --guardian-respawn 零断流复用内核）
        if in_lightweight {
            launch_args.push("--silent".to_string());
        }

        let work_dir = args.target_exe.parent().filter(|p| p.is_dir());
        let spawn_res = spawn_isolated_process_with(
            &args.target_exe,
            |_| launch_args.clone(),
            work_dir,
            true,
            in_lightweight,
        );

        match spawn_res {
            Ok(respawned) => {
                append_structured_guardian_log(
                    &args.log_dir,
                    "WARN",
                    session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    next_gen,
                    "GUARD_RESPAWN_OK",
                    &format!(
                        "伴生看门狗已完成自愈拉起 (new_pid={}, gen={}, broker={}, in_job={:?}, degraded={}, mode={}, target={})",
                        respawned.pid,
                        next_gen,
                        respawned.broker,
                        respawned.in_job,
                        respawned.degraded,
                        if in_lightweight { "silent" } else { "gui" },
                        args.target_exe.display()
                    ),
                );
            }
            Err(err) => {
                append_structured_guardian_log(
                    &args.log_dir,
                    "ERROR",
                    session_id,
                    args.parent_pid,
                    Some(guard_pid),
                    next_gen,
                    "GUARD_RESPAWN_FAILED",
                    &format!(
                        "自愈拉起主进程失败 (mode={}): {err}",
                        if in_lightweight { "silent" } else { "gui" }
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::{
        BROKER_WMI_SESSION0, GUARD_HEARTBEAT_FILENAME, GUARDIAN_LOG_FILENAME, GuardHeartbeat, GuardianArgs,
        INTENTIONAL_EXIT_CONFIRMED_TTL_MS, INTENTIONAL_EXIT_FILENAME, INTENTIONAL_EXIT_PROOF_TTL_MS,
        ISOLATED_GUARDIAN_BINARY_NAME, IntentionalExitProof, IntentionalProofCheck, MAIN_HEARTBEAT_FILENAME,
        MAX_RAPID_RESPAWN_GEN, MainHeartbeat, RESPAWN_SAFETY_DELAY_MS, RESPAWN_STATE_FILENAME, RespawnInfo,
        RespawnSessionState, append_forensic_log, append_structured_guardian_log, clear_intentional_exit_proof_in,
        compute_guard_rebuild_backoff_ms, compute_next_generation, current_unix_ms, evaluate_and_record_respawn_in,
        evaluate_intentional_exit_proof, evaluate_respawn_budget, format_exit_code_diagnosis,
        parse_guardian_args_from_iter, parse_respawn_info_from_iter, parse_respawn_session_epoch_from_iter,
        read_intentional_exit_proof_from, read_json_file, read_respawn_state_from,
        reset_respawn_state_for_clean_start_in, sync_guardian_binary_to_dir, write_guard_heartbeat_to,
        write_intentional_exit_proof_to, write_json_atomic, write_main_heartbeat_to,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_guardian_args_with_and_without_target_exe() {
        let args = [
            r"C:\Users\demo\AppData\Local\guard\clash-mini-guard.exe",
            "--guardian",
            "--parent-pid",
            "13712",
            "--target-exe",
            r"C:\Program Files\Clash Mini\clash-mini.exe",
            "--log-dir",
            r"C:\Temp\logs",
            "--guard-dir",
            r"C:\Temp\guard",
            "--singleton-port",
            "33335",
            "--gen",
            "2",
            "--session-id",
            "s-13712-999",
            "--session-epoch",
            "epoch-001",
            "--broker",
            BROKER_WMI_SESSION0,
        ];
        let parsed = parse_guardian_args_from_iter(args);
        assert_eq!(
            parsed,
            Some(GuardianArgs {
                parent_pid: 13712,
                target_exe: PathBuf::from(r"C:\Program Files\Clash Mini\clash-mini.exe"),
                log_dir: PathBuf::from(r"C:\Temp\logs"),
                guard_dir: Some(PathBuf::from(r"C:\Temp\guard")),
                singleton_port: 33335,
                generation: 2,
                session_id: Some("s-13712-999".to_string()),
                session_epoch: Some("epoch-001".to_string()),
                broker: Some(BROKER_WMI_SESSION0.to_string()),
            })
        );
        assert_eq!(
            parsed.as_ref().map(GuardianArgs::resolved_session_epoch),
            Some("epoch-001".to_string())
        );

        let legacy_args = [
            r"C:\Program Files\Clash Mini\clash-mini.exe",
            "--guardian",
            "--parent-pid",
            "8888",
            "--log-dir",
            r"C:\Temp\logs",
        ];
        let legacy_parsed = parse_guardian_args_from_iter(legacy_args);
        assert_eq!(
            legacy_parsed.as_ref().map(|p| &p.target_exe),
            Some(&PathBuf::from(r"C:\Program Files\Clash Mini\clash-mini.exe"))
        );
        assert_eq!(
            legacy_parsed.as_ref().map(GuardianArgs::resolved_session_id),
            Some("pid-8888".to_string())
        );
        assert_eq!(
            legacy_parsed.as_ref().map(GuardianArgs::resolved_session_epoch),
            Some("pid-8888".to_string())
        );
        assert_eq!(parse_guardian_args_from_iter(["clash-mini.exe", "--silent"]), None);
    }

    #[test]
    fn test_parse_respawn_info_and_session_epoch() {
        let gui_args = [
            "clash-mini.exe",
            "--guardian-respawn",
            "--guardian-gen",
            "2",
            "--session-epoch",
            "epoch-abc",
            "--prev-pid",
            "9527",
            "--prev-exit",
            "99",
        ];
        assert_eq!(
            parse_respawn_info_from_iter(gui_args),
            Some(RespawnInfo {
                generation: 2,
                prev_pid: Some(9527),
                prev_exit_code: Some(99),
                silent: false,
            })
        );
        assert_eq!(
            parse_respawn_session_epoch_from_iter(gui_args),
            Some("epoch-abc".to_string())
        );

        let silent_args = [
            "clash-mini.exe",
            "--guardian-respawn",
            "--guardian-gen",
            "1",
            "--silent",
        ];
        assert_eq!(
            parse_respawn_info_from_iter(silent_args),
            Some(RespawnInfo {
                generation: 1,
                prev_pid: None,
                prev_exit_code: None,
                silent: true,
            })
        );
        assert_eq!(parse_respawn_session_epoch_from_iter(silent_args), None);
        assert_eq!(parse_respawn_info_from_iter(["clash-mini.exe"]), None);
    }

    #[test]
    fn test_compute_next_generation_cumulative_circuit_breaker() {
        assert_eq!(MAX_RAPID_RESPAWN_GEN, 3);
        const {
            assert!(RESPAWN_SAFETY_DELAY_MS >= 3500);
        }
        assert_eq!(compute_next_generation(0), Some(1));
        assert_eq!(compute_next_generation(1), Some(2));
        assert_eq!(compute_next_generation(2), Some(MAX_RAPID_RESPAWN_GEN));
        // C3: 严禁随时间推移清零，同一开机与会话纪元达 3 次上限后一律熔断
        assert_eq!(compute_next_generation(MAX_RAPID_RESPAWN_GEN), None);
        assert_eq!(compute_next_generation(MAX_RAPID_RESPAWN_GEN + 5), None);
    }

    #[test]
    fn test_evaluate_respawn_budget_boot_epoch_and_fail_closed() {
        let dir = std::env::temp_dir().join(format!("clash_mini_respawn_state_test_{}", std::process::id()));
        let log_dir = dir.join("logs");
        let _ = std::fs::remove_dir_all(&dir);

        let base_ms: u64 = 1_760_000_000_000;
        let exhausted_state = RespawnSessionState {
            session_epoch_id: "epoch-a".to_string(),
            boot_fingerprint: "boot:1760000000".to_string(),
            respawn_count: MAX_RAPID_RESPAWN_GEN,
            last_respawn_unix_ms: base_ms,
            last_parent_pid: 1001,
        };
        // C3: 同一次开机（容许 ±30s 时钟抖动）无论隔多久都不复位预算
        assert_eq!(
            evaluate_respawn_budget(&exhausted_state, MAX_RAPID_RESPAWN_GEN, "epoch-a", "boot:1760000002"),
            None
        );
        // 纪元不一致说明用户已发起后续干净冷启动，旧纪元停止干预
        assert_eq!(
            evaluate_respawn_budget(&exhausted_state, 0, "epoch-old", "boot:1760000002"),
            None
        );
        // 跨真实 OS 重启（boot_fingerprint 跃变 > 30s）自动复位为第 1 代
        assert_eq!(
            evaluate_respawn_budget(&exhausted_state, MAX_RAPID_RESPAWN_GEN, "epoch-a", "boot:1760005000"),
            Some(1)
        );

        // 同一 epoch 连续累计 1 -> 2 -> 3 -> 熔断
        reset_respawn_state_for_clean_start_in(&dir, Some(&log_dir), "epoch-a", "boot:1760000000");
        assert_eq!(
            evaluate_and_record_respawn_in(&dir, "epoch-a", 0, 100, base_ms + 10_000, "boot:1760000000", true),
            Ok(Some(1))
        );
        assert_eq!(
            evaluate_and_record_respawn_in(&dir, "epoch-a", 1, 101, base_ms + 4_000_000, "boot:1760000001", true),
            Ok(Some(2))
        );
        assert_eq!(
            evaluate_and_record_respawn_in(&dir, "epoch-a", 2, 102, base_ms + 9_000_000, "boot:1760000001", true),
            Ok(Some(3))
        );
        assert_eq!(
            evaluate_and_record_respawn_in(&dir, "epoch-a", 3, 103, base_ms + 99_000_000, "boot:1760000001", true),
            Ok(None)
        );

        // C15 + C3: 干净冷启动复位前序计数时必须在 guardian.log 留下审计记录
        reset_respawn_state_for_clean_start_in(&dir, Some(&log_dir), "epoch-b", "boot:1760000001");
        assert_eq!(read_respawn_state_from(&dir).respawn_count, 0);
        let log_txt = std::fs::read_to_string(log_dir.join(GUARDIAN_LOG_FILENAME)).unwrap_or_default();
        assert!(log_txt.contains("RESPAWN_BUDGET_RESET_ON_CLEAN_START"));
        assert!(log_txt.contains("cleared_count=3"));
        assert_eq!(
            evaluate_and_record_respawn_in(&dir, "epoch-b", 0, 200, base_ms + 99_010_000, "boot:1760000001", false),
            Ok(Some(1))
        );

        // C4: 状态文件 JSON 损坏时必须 Fail-Closed 返回 Err 拒绝拉起
        let _ = std::fs::write(dir.join(RESPAWN_STATE_FILENAME), b"{corrupt_json");
        assert!(
            evaluate_and_record_respawn_in(&dir, "epoch-b", 0, 200, base_ms + 99_020_000, "boot:1760000001", false)
                .is_err()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compute_guard_rebuild_backoff_ms() {
        assert_eq!(compute_guard_rebuild_backoff_ms(0), 1_500);
        assert_eq!(compute_guard_rebuild_backoff_ms(1), 2_000);
        assert_eq!(compute_guard_rebuild_backoff_ms(2), 5_000);
        assert_eq!(compute_guard_rebuild_backoff_ms(3), 15_000);
        assert_eq!(compute_guard_rebuild_backoff_ms(4), 30_000);
        assert_eq!(compute_guard_rebuild_backoff_ms(99), 30_000);
    }

    #[test]
    fn test_sync_guardian_binary_to_dir_stages_reuses_and_updates() {
        let base_dir = std::env::temp_dir().join(format!("clash_mini_guard_sync_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base_dir);
        let src_dir = base_dir.join("inst");
        let guard_dir = base_dir.join("guard");
        let _ = std::fs::create_dir_all(&src_dir);
        let _ = std::fs::create_dir_all(&guard_dir);

        let fake_main = src_dir.join("clash-mini.exe");
        let stale_stage = guard_dir.join("clash-mini-guard.staging.999999.exe");
        let _ = std::fs::write(&fake_main, b"MZ-v1-binary-payload");
        let _ = std::fs::write(&stale_stage, b"leftover");

        let synced = sync_guardian_binary_to_dir(&fake_main, &guard_dir);
        assert!(synced.is_ok());
        if let Ok(guard_exe) = synced {
            assert_eq!(
                guard_exe.file_name().and_then(|s| s.to_str()),
                Some(ISOLATED_GUARDIAN_BINARY_NAME)
            );
            assert!(!stale_stage.exists());
            assert_eq!(
                std::fs::read(&guard_exe).unwrap_or_default(),
                b"MZ-v1-binary-payload".to_vec()
            );

            // 未变更再次同步命中快速复用路径
            let reused = sync_guardian_binary_to_dir(&fake_main, &guard_dir);
            assert_eq!(reused.ok(), Some(guard_exe.clone()));

            // 相同字节长度但内容变更（补丁构建），SHA-256 校验必须触发原子更新
            let _ = std::fs::write(&fake_main, b"MZ-v2-binary-payload");
            let updated_same_len = sync_guardian_binary_to_dir(&fake_main, &guard_dir);
            assert_eq!(updated_same_len.ok(), Some(guard_exe.clone()));
            assert_eq!(
                std::fs::read(&guard_exe).unwrap_or_default(),
                b"MZ-v2-binary-payload".to_vec()
            );

            // 模拟升级后主二进制体积变化，应自动原子更新看门狗副本
            let _ = std::fs::write(&fake_main, b"MZ-v3-upgraded-binary-payload-longer");
            let updated = sync_guardian_binary_to_dir(&fake_main, &guard_dir);
            assert_eq!(updated.ok(), Some(guard_exe.clone()));
            assert_eq!(
                std::fs::read(&guard_exe).unwrap_or_default(),
                b"MZ-v3-upgraded-binary-payload-longer".to_vec()
            );
        }

        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn test_format_exit_code_diagnosis() {
        let msg99 = format_exit_code_diagnosis(99);
        assert!(msg99.contains("360") && msg99.contains("0x63"));
        let msg_av = format_exit_code_diagnosis(0xC000_0005);
        assert!(msg_av.contains("STATUS_ACCESS_VIOLATION"));
    }

    #[test]
    fn test_append_forensic_log_and_non_silent_fallback() {
        let base = std::env::temp_dir().join(format!("clash_mini_guardian_log_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let ok_dir = base.join("ok_logs");
        append_forensic_log(&ok_dir, "ERROR", "unit test forensic message");
        let content = std::fs::read_to_string(ok_dir.join(GUARDIAN_LOG_FILENAME)).unwrap_or_default();
        assert!(content.contains("ERROR"));
        assert!(content.contains("event=FORENSIC"));
        assert!(content.contains("unit test forensic message"));
        assert!(!ok_dir.join("latest.log").exists());

        // C7: 将 log_dir 阻塞为普通文件，验证写入失败时触发 temp 回退落盘而非静默丢失
        let _ = std::fs::create_dir_all(&base);
        let blocked_log_dir = base.join("file_blocking_dir");
        let _ = std::fs::write(&blocked_log_dir, b"not_a_directory");
        let marker = format!("fallback-marker-{}", std::process::id());
        append_structured_guardian_log(&blocked_log_dir, "ERROR", "s-fb", 1, None, 0, "TEST_FALLBACK", &marker);
        let fallback_path = std::env::temp_dir().join("clash-mini-guard-fallback.log");
        let fb_content = std::fs::read_to_string(&fallback_path).unwrap_or_default();
        assert!(fb_content.contains(&marker));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_intentional_exit_proof_ttl_and_strict_matching() {
        let dir = std::env::temp_dir().join(format!("clash_mini_guard_proof_ttl_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let now_ms: u64 = 1_760_000_000_000;
        let proof = IntentionalExitProof {
            pid: 4242,
            session_id: "s-4242-1".to_string(),
            phase: "prepared".to_string(),
            timestamp_unix_ms: now_ms,
            timestamp_local: "2026-11-13 12:00:00.000".to_string(),
            reason: "prepare_exit".to_string(),
        };
        assert!(write_intentional_exit_proof_to(&dir, &proof).is_ok());
        assert_eq!(read_intentional_exit_proof_from(&dir), Some(proof.clone()));

        // C2: 必须 pid && session_id 同时匹配（拒绝旧或跨会话 PID 重用误放行）
        assert!(matches!(
            evaluate_intentional_exit_proof(&dir, 4242, "s-4242-other", now_ms + 1_000),
            IntentionalProofCheck::Rejected { .. }
        ));
        assert!(matches!(
            evaluate_intentional_exit_proof(&dir, 9999, "s-4242-1", now_ms + 1_000),
            IntentionalProofCheck::Rejected { .. }
        ));

        // C1: prepared 阶段 30s TTL 内有效，超时后拒绝并自动清理
        assert_eq!(
            evaluate_intentional_exit_proof(&dir, 4242, "s-4242-1", now_ms + INTENTIONAL_EXIT_PROOF_TTL_MS),
            IntentionalProofCheck::Valid {
                phase: "prepared".to_string(),
                reason: "prepare_exit".to_string(),
            }
        );
        assert!(matches!(
            evaluate_intentional_exit_proof(&dir, 4242, "s-4242-1", now_ms + INTENTIONAL_EXIT_PROOF_TTL_MS + 1),
            IntentionalProofCheck::Rejected { .. }
        ));
        assert!(!dir.join(INTENTIONAL_EXIT_FILENAME).exists());

        // C1: confirmed 阶段适用 120s TTL
        let confirmed = IntentionalExitProof {
            phase: "confirmed".to_string(),
            reason: "run_event_exit".to_string(),
            ..proof
        };
        assert!(write_intentional_exit_proof_to(&dir, &confirmed).is_ok());
        assert_eq!(
            evaluate_intentional_exit_proof(&dir, 4242, "s-4242-1", now_ms + INTENTIONAL_EXIT_CONFIRMED_TTL_MS - 10),
            IntentionalProofCheck::Valid {
                phase: "confirmed".to_string(),
                reason: "run_event_exit".to_string(),
            }
        );
        assert!(matches!(
            evaluate_intentional_exit_proof(&dir, 4242, "s-4242-1", now_ms + INTENTIONAL_EXIT_CONFIRMED_TTL_MS + 1),
            IntentionalProofCheck::Rejected { .. }
        ));
        assert!(!dir.join(INTENTIONAL_EXIT_FILENAME).exists());

        // 损坏 JSON 自动拒绝并清理
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(INTENTIONAL_EXIT_FILENAME), b"not-json");
        assert!(matches!(
            evaluate_intentional_exit_proof(&dir, 4242, "s-4242-1", now_ms),
            IntentionalProofCheck::Rejected { .. }
        ));
        assert!(!dir.join(INTENTIONAL_EXIT_FILENAME).exists());

        let main_hb = MainHeartbeat {
            pid: 4242,
            session_id: "s-4242-1".to_string(),
            in_lightweight: true,
            is_exiting: false,
            parent_in_job: Some(false),
            breakaway_degraded: false,
            spawn_broker: Some(BROKER_WMI_SESSION0.to_string()),
            guard_pid: Some(5252),
            last_seen_unix_ms: 1_760_000_015_000,
            last_seen_local: "2026-11-13 12:00:15.000".to_string(),
        };
        assert!(write_main_heartbeat_to(&dir, &main_hb).is_ok());
        assert!(dir.join(MAIN_HEARTBEAT_FILENAME).exists());

        let guard_hb = GuardHeartbeat {
            pid: 5252,
            parent_pid: 4242,
            session_id: "s-4242-1".to_string(),
            generation: 1,
            guard_in_job: Some(false),
            spawn_broker: Some(BROKER_WMI_SESSION0.to_string()),
            status: "watching".to_string(),
            last_seen_unix_ms: 1_760_000_015_000,
            last_seen_local: "2026-11-13 12:00:15.000".to_string(),
        };
        assert!(write_guard_heartbeat_to(&dir, &guard_hb).is_ok());
        assert!(dir.join(GUARD_HEARTBEAT_FILENAME).exists());

        clear_intentional_exit_proof_in(&dir);
        assert_eq!(read_intentional_exit_proof_from(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_concurrent_write_json_atomic_and_live_target_preservation() {
        let dir = std::env::temp_dir().join(format!("clash_mini_atomic_json_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join(RESPAWN_STATE_FILENAME);

        // C5: 8 线程并发写入同一目标文件，零报错、零 .tmp 残留且最终 JSON 可正常解析
        let handles: Vec<_> = (0..8u32)
            .map(|tid| {
                let p = target.clone();
                std::thread::spawn(move || {
                    for iter in 0..12u32 {
                        let st = RespawnSessionState {
                            session_epoch_id: format!("epoch-{tid}-{iter}"),
                            boot_fingerprint: "boot:100".to_string(),
                            respawn_count: iter % 3,
                            last_respawn_unix_ms: current_unix_ms(),
                            last_parent_pid: tid + 1000,
                        };
                        assert!(write_json_atomic(&p, &st).is_ok());
                    }
                })
            })
            .collect();
        for h in handles {
            assert!(h.join().is_ok());
        }
        assert!(read_json_file::<RespawnSessionState>(&target).is_some());
        let leftover_tmp = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().contains(".tmp."));
        assert!(!leftover_tmp);

        // C6: 模拟替换目标被独占打开失败时，旧现网文件必须完好保留（绝不先删后移造成丢状态空窗）
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt as _;
            let baseline = RespawnSessionState {
                session_epoch_id: "live-baseline".to_string(),
                boot_fingerprint: "boot:200".to_string(),
                respawn_count: 2,
                last_respawn_unix_ms: 1_760_000_000_000,
                last_parent_pid: 7777,
            };
            assert!(write_json_atomic(&target, &baseline).is_ok());
            if let Ok(lock) = std::fs::OpenOptions::new().read(true).share_mode(0).open(&target) {
                let replacement = RespawnSessionState {
                    session_epoch_id: "should-not-wipe".to_string(),
                    ..baseline.clone()
                };
                assert!(write_json_atomic(&target, &replacement).is_err());
                drop(lock);
                assert_eq!(read_json_file::<RespawnSessionState>(&target), Some(baseline));
            }
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 场景 6 E2E 辅助子进程（仅当 `CLASH_MINI_S6_DIR` 存在时运行，日常 `cargo test` 立即返回）。
    #[cfg(windows)]
    #[test]
    fn scenario6_trapped_main_helper() {
        let Ok(dir_str) = std::env::var("CLASH_MINI_S6_DIR") else {
            return;
        };
        let Ok(event_name) = std::env::var("CLASH_MINI_S6_EVENT") else {
            return;
        };
        let dir = PathBuf::from(dir_str);

        unsafe extern "system" {
            fn OpenEventW(dw_desired_access: u32, b_inherit_handle: i32, lp_name: *const u16) -> *mut std::ffi::c_void;
            fn WaitForSingleObject(h_handle: *mut std::ffi::c_void, dw_milliseconds: u32) -> u32;
            fn CloseHandle(h_object: *mut std::ffi::c_void) -> i32;
        }
        const SYNCHRONIZE: u32 = 0x0010_0000;
        let event_w = super::win32_impl::to_wide_null(&event_name);
        // SAFETY: 打开测试父进程创建的本地同步命名事件，等待其完成 AssignProcessToJobObject 后再唤醒。
        unsafe {
            let ev = OpenEventW(SYNCHRONIZE, 0, event_w.as_ptr());
            assert!(!ev.is_null(), "failed to open scenario6 sync event");
            assert_eq!(WaitForSingleObject(ev, 10_000), 0);
            let _ = CloseHandle(ev);
        }

        // 1. 验证当前子进程确实被囚禁在父测试创建的无 Breakaway Job 内
        assert_eq!(super::win32_impl::query_current_process_in_job(), Some(true));

        // 2. 验证传统 CREATE_BREAKAWAY_FROM_JOB 在无 BREAKAWAY_OK 的 Job 下必然失败
        {
            use std::os::windows::process::CommandExt as _;
            const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let breakaway_res = std::process::Command::new(r"C:\Windows\System32\PING.EXE")
                .args(["-n", "2", "127.0.0.1"])
                .creation_flags(CREATE_BREAKAWAY_FROM_JOB | CREATE_NO_WINDOW)
                .spawn();
            assert!(
                breakaway_res.is_err(),
                "CREATE_BREAKAWAY_FROM_JOB unexpectedly succeeded inside non-breakaway Job"
            );
        }

        // 3. 调用生产 A6 隔离拉起通道（allow_degraded=false：必须真逃逸成功）
        let spawned = super::win32_impl::spawn_isolated_process_with(
            std::path::Path::new(r"C:\Windows\System32\PING.EXE"),
            |_| vec!["-n".to_string(), "45".to_string(), "127.0.0.1".to_string()],
            None,
            false,
            true,
        )
        .unwrap_or_else(|e| panic!("A6 isolated spawn failed inside trapped Job: {e}"));

        assert!(!spawned.degraded);
        assert_eq!(spawned.in_job, Some(false));
        assert!(
            spawned.broker == BROKER_WMI_SESSION0 || spawned.broker == super::BROKER_TASK_SCHEDULER_COM,
            "unexpected broker: {}",
            spawned.broker
        );

        let payload = serde_json::json!({
            "trapped_pid": std::process::id(),
            "companion_pid": spawned.pid,
            "broker": spawned.broker,
        });
        let _ = std::fs::write(dir.join("s6_result.json"), payload.to_string());
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    /// 场景 6 真实端到端验证：
    /// 在非 Breakaway Windows Job 内模拟主进程被强制 assign -> `TerminateJobObject` 整树强杀后，
    /// 验证经 A6 Broker 拉起的独立进程真正在该 Job 之外（`IsProcessInJob == false`）且完整存活。
    #[cfg(windows)]
    #[test]
    fn test_scenario6_job_termination_wmi_broker_escape() {
        use std::os::windows::io::AsRawHandle as _;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
            TerminateJobObject,
        };

        unsafe extern "system" {
            fn CreateEventW(
                lp_event_attributes: *const std::ffi::c_void,
                b_manual_reset: i32,
                b_initial_state: i32,
                lp_name: *const u16,
            ) -> *mut std::ffi::c_void;
            fn SetEvent(h_event: *mut std::ffi::c_void) -> i32;
            fn OpenProcess(dw_desired_access: u32, b_inherit_handle: i32, dw_process_id: u32) -> *mut std::ffi::c_void;
            fn TerminateProcess(h_process: *mut std::ffi::c_void, u_exit_code: u32) -> i32;
            fn WaitForSingleObject(h_handle: *mut std::ffi::c_void, dw_milliseconds: u32) -> u32;
            fn CloseHandle(h_object: *mut std::ffi::c_void) -> i32;
        }
        const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
        const PROCESS_TERMINATE: u32 = 0x0001;
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const WAIT_TIMEOUT: u32 = 0x0000_0102;

        let dir = std::env::temp_dir().join(format!("clash_mini_s6_e2e_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let result_path = dir.join("s6_result.json");
        let event_name = format!(
            "Local\\ClashMiniS6Assigned_{}_{}",
            std::process::id(),
            current_unix_ms()
        );
        let event_w = super::win32_impl::to_wide_null(&event_name);

        // SAFETY: 创建仅含 KILL_ON_JOB_CLOSE、显式禁止 Breakaway 的真实 Win32 Job Object 与同步事件。
        unsafe {
            let h_ready = CreateEventW(std::ptr::null(), 1, 0, event_w.as_ptr());
            assert!(!h_ready.is_null());

            let h_job = CreateJobObjectW(None, None).unwrap_or_else(|e| panic!("CreateJobObjectW failed: {e}"));
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                h_job,
                JobObjectExtendedLimitInformation,
                (&raw const info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .unwrap_or_else(|e| panic!("SetInformationJobObject failed: {e}"));

            let exe = std::env::current_exe().unwrap_or_else(|e| panic!("current_exe failed: {e}"));
            let mut trapped_child = std::process::Command::new(exe)
                .args([
                    "--exact",
                    "process::guardian::tests::scenario6_trapped_main_helper",
                    "--nocapture",
                ])
                .env("CLASH_MINI_S6_DIR", &dir)
                .env("CLASH_MINI_S6_EVENT", &event_name)
                .spawn()
                .unwrap_or_else(|e| panic!("spawn trapped child failed: {e}"));

            AssignProcessToJobObject(h_job, HANDLE(trapped_child.as_raw_handle()))
                .unwrap_or_else(|e| panic!("AssignProcessToJobObject failed: {e}"));
            assert_ne!(SetEvent(h_ready), 0);

            // 等待被困子进程完成 Job 检查并经 WMI/TaskScheduler Broker 拉起独立伴生进程
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            let mut companion_pid: Option<u32> = None;
            while std::time::Instant::now() < deadline {
                if let Ok(bytes) = std::fs::read(&result_path)
                    && let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes)
                {
                    companion_pid = val
                        .get("companion_pid")
                        .and_then(serde_json::Value::as_u64)
                        .map(|p| p as u32);
                    if companion_pid.is_some() {
                        break;
                    }
                }
                if let Ok(Some(status)) = trapped_child.try_wait() {
                    let _ = CloseHandle(h_ready);
                    let _ = TerminateJobObject(h_job, 99);
                    let _ = CloseHandle(h_job.0);
                    let _ = std::fs::remove_dir_all(&dir);
                    panic!("trapped child exited early before spawning companion: {status}");
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let _ = CloseHandle(h_ready);

            let companion_pid = companion_pid.unwrap_or_else(|| {
                let _ = TerminateJobObject(h_job, 99);
                let _ = trapped_child.wait();
                let _ = CloseHandle(h_job.0);
                let _ = std::fs::remove_dir_all(&dir);
                panic!("timed out waiting for scenario6 companion spawn");
            });

            let h_companion = OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                companion_pid,
            );
            assert!(
                !h_companion.is_null(),
                "OpenProcess on escaped companion {companion_pid} failed"
            );
            assert_eq!(super::win32_impl::query_handle_in_job(h_companion), Some(false));

            // 核心破坏性验证：TerminateJobObject(h_job, 99) 整树强杀主进程 Job
            TerminateJobObject(h_job, 99).unwrap_or_else(|e| panic!("TerminateJobObject failed: {e}"));
            let _ = CloseHandle(h_job.0);

            let wait_trapped = trapped_child.wait();
            assert_eq!(wait_trapped.ok().and_then(|s| s.code()), Some(99));

            // 断言独立进程逃逸成功：主 Job 死绝后 500ms 仍然存活且不在任何 Job 内（或不在被杀 Job）
            let alive_res = WaitForSingleObject(h_companion, 500);
            assert_eq!(
                alive_res, WAIT_TIMEOUT,
                "escaped companion died after TerminateJobObject!"
            );
            assert_eq!(super::win32_impl::query_handle_in_job(h_companion), Some(false));

            // 清理临时伴生验证进程与目录
            let _ = TerminateProcess(h_companion, 0);
            let _ = WaitForSingleObject(h_companion, 3_000);
            let _ = CloseHandle(h_companion);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn test_sentinel_window_satisfies_anti_kill_invariants() {
        use std::ffi::c_void;
        unsafe extern "system" {
            fn IsWindowVisible(h_wnd: *mut c_void) -> i32;
            fn GetWindowLongPtrW(h_wnd: *mut c_void, n_index: i32) -> isize;
            fn GetWindowTextLengthW(h_wnd: *mut c_void) -> i32;
            fn GetLayeredWindowAttributes(
                hwnd: *mut c_void,
                pcr_key: *mut u32,
                pb_alpha: *mut u8,
                pdw_flags: *mut u32,
            ) -> i32;
            fn MonitorFromWindow(hwnd: *mut c_void, dw_flags: u32) -> *mut c_void;
            fn SendMessageW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> isize;
            fn DestroyWindow(h_wnd: *mut c_void) -> i32;
        }
        const GWL_EXSTYLE: i32 = -20;
        const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
        const WS_EX_TOPMOST: isize = 0x0000_0008;
        const WS_EX_APPWINDOW: isize = 0x0004_0000;
        const MONITOR_DEFAULTTONULL: u32 = 0;
        const WM_NCHITTEST: u32 = 0x0084;
        const HTTRANSPARENT: isize = -1;

        let pair =
            super::win32_impl::create_sentinel_window_pair("ClashMiniResidentHostTest", "Clash Mini Sentinel Test");
        assert!(pair.is_some());
        if let Some((owner, sentinel)) = pair {
            unsafe {
                // Owner 必须不可见（保证哨兵窗不出现在任务栏与 Alt+Tab）
                assert_eq!(IsWindowVisible(owner), 0);
                // Sentinel 必须满足 IsWindowVisible == TRUE 且不带 TOOLWINDOW / TOPMOST / APPWINDOW
                assert_ne!(IsWindowVisible(sentinel), 0);
                let ex_style = GetWindowLongPtrW(sentinel, GWL_EXSTYLE);
                assert_eq!(ex_style & WS_EX_TOOLWINDOW, 0);
                assert_eq!(ex_style & WS_EX_TOPMOST, 0);
                assert_eq!(ex_style & WS_EX_APPWINDOW, 0);

                // 必须落在真实显示器工作区内（MonitorFromWindow != NULL，排除 -32000 屏外最小化特征）
                let h_mon = MonitorFromWindow(sentinel, MONITOR_DEFAULTTONULL);
                assert!(!h_mon.is_null());

                // 必须具备非空标题与非零 Alpha (1/255)，且命中测试完全穿透 (HTTRANSPARENT)
                assert!(GetWindowTextLengthW(sentinel) > 0);
                let mut alpha: u8 = 0;
                let mut flags: u32 = 0;
                assert_ne!(
                    GetLayeredWindowAttributes(sentinel, std::ptr::null_mut(), &raw mut alpha, &raw mut flags),
                    0
                );
                assert_eq!(alpha, super::win32_impl::SENTINEL_VISIBLE_ALPHA);
                assert_eq!(SendMessageW(sentinel, WM_NCHITTEST, 0, 0), HTTRANSPARENT);

                let _ = DestroyWindow(sentinel);
                let _ = DestroyWindow(owner);
            }
        }
    }
}
