use std::{
    fs::{self, OpenOptions},
    io::{self, Write as _},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::{Duration, SystemTime},
};

use clash_verge_logging::{Type, logging};

/// 短时间（`< STABLE_UPTIME_RESET_SECS`）内连续异常终止的最大自动拉起代数，防止真实启动级故障引发无限拉起死循环。
pub const MAX_RAPID_RESPAWN_GEN: u32 = 3;

/// 主进程稳定存活超过该秒数（120s）后，若再遭外部强杀，视为独立外部清理事件，拉起代数重置为 1。
pub const STABLE_UPTIME_RESET_SECS: u64 = 120;

/// 物理隔离目录中的伴生看门狗主可执行文件名（与安装目录 `clash-mini.exe` 异名异目录，防应用组连坐强杀）。
pub const ISOLATED_GUARDIAN_BINARY_NAME: &str = "clash-mini-guard.exe";

/// 当主隔离文件被短暂锁定时使用的备用隔离可执行文件名。
pub const ISOLATED_GUARDIAN_FALLBACK_NAME: &str = "clash-mini-guard-alt.exe";

/// 检测到主进程在轻量模式下被外部强杀后，拉起前的安全避让等待毫秒数（避让安装/卸载器与整组清理）。
const RESPAWN_SAFETY_DELAY_MS: u64 = 1500;

/// 伴生看门狗稳定存活超过该秒数后被杀，视为独立误杀事件，快速重建退避计数归零。
const GUARD_RAPID_DEATH_WINDOW_SECS: u64 = 60;

/// 伴生看门狗首次或稳态后意外终止的基础重建延迟毫秒数。
const GUARD_REBUILD_BASE_DELAY_MS: u64 = 1500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardianArgs {
    pub parent_pid: u32,
    pub target_exe: PathBuf,
    pub log_dir: PathBuf,
    pub singleton_port: u16,
    pub generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnInfo {
    pub generation: u32,
    pub prev_pid: Option<u32>,
    pub prev_exit_code: Option<u32>,
}

/// 判断当前进程是否由 `--guardian` 伴生看门狗静默拉起。
pub fn is_guardian_respawn() -> bool {
    static IS_RESPAWN: OnceLock<bool> = OnceLock::new();
    *IS_RESPAWN.get_or_init(|| std::env::args().any(|arg| arg == "--guardian-respawn"))
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
    let mut singleton_port: u16 = 33335;
    let mut generation: u32 = 0;

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
        singleton_port,
        generation,
    })
}

/// 根据父进程本次存活时长与当前代数计算下一代拉起计数。
/// 返回 `None` 表示短时间内连续异常已超过上限，禁止继续拉起。
pub const fn compute_next_generation(current_gen: u32, uptime_secs: u64) -> Option<u32> {
    let next_gen = if uptime_secs >= STABLE_UPTIME_RESET_SECS {
        1
    } else {
        current_gen.saturating_add(1)
    };
    if next_gen > MAX_RAPID_RESPAWN_GEN {
        None
    } else {
        Some(next_gen)
    }
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

fn system_times_within_2s(a: Option<SystemTime>, b: Option<SystemTime>) -> bool {
    let (Some(ta), Some(tb)) = (a, b) else {
        return false;
    };
    let diff = ta.duration_since(tb).or_else(|_| tb.duration_since(ta));
    diff.is_ok_and(|d| d <= Duration::from_secs(2))
}

fn is_guardian_binary_up_to_date(src_meta: &fs::Metadata, dst_path: &Path) -> bool {
    let Ok(dst_meta) = fs::metadata(dst_path) else {
        return false;
    };
    dst_meta.is_file()
        && dst_meta.len() == src_meta.len()
        && system_times_within_2s(src_meta.modified().ok(), dst_meta.modified().ok())
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
/// 若目标已存在且体积与修改时间一致则零磁盘写入直接复用；升级后首次启动经 `.staging` 原子替换。
pub fn sync_guardian_binary_to_dir(main_exe: &Path, guard_dir: &Path) -> io::Result<PathBuf> {
    let src_meta = fs::metadata(main_exe)?;
    if !src_meta.is_file() || src_meta.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "main executable metadata is invalid or empty",
        ));
    }

    fs::create_dir_all(guard_dir)?;
    cleanup_stale_guard_staging(guard_dir);

    let primary_guard = guard_dir.join(ISOLATED_GUARDIAN_BINARY_NAME);
    if is_guardian_binary_up_to_date(&src_meta, &primary_guard) {
        return Ok(primary_guard);
    }

    let staging = guard_dir.join(format!("clash-mini-guard.staging.{}.exe", std::process::id()));
    let copied_bytes = fs::copy(main_exe, &staging)?;
    if copied_bytes != src_meta.len() {
        let _ = fs::remove_file(&staging);
        return Err(io::Error::new(
            io::ErrorKind::WriteZero,
            "incomplete copy while staging guardian binary",
        ));
    }

    if fs::rename(&staging, &primary_guard).is_ok() {
        return Ok(primary_guard);
    }

    // 若上一代看门狗正处于微秒级收尾退出尚未释放映像锁，短暂避让 80ms 后重试一次，仍占用则落入备用隔离文件名
    std::thread::sleep(Duration::from_millis(80));
    if fs::rename(&staging, &primary_guard).is_ok() {
        return Ok(primary_guard);
    }

    let fallback_guard = guard_dir.join(ISOLATED_GUARDIAN_FALLBACK_NAME);
    if is_guardian_binary_up_to_date(&src_meta, &fallback_guard) {
        let _ = fs::remove_file(&staging);
        return Ok(fallback_guard);
    }
    match fs::rename(&staging, &fallback_guard) {
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

/// 向日志目录的 `latest.log` 同步追加一行标准格式的取证日志并强制刷盘。
pub fn append_forensic_log(log_dir: &Path, level: &str, message: &str) {
    let _ = fs::create_dir_all(log_dir);
    let log_file = log_dir.join("latest.log");
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{now}] {level} [System] [Guardian] {message}\n");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file) {
        let _ = file.write_all(line.as_bytes());
        let _ = file.sync_data();
    }
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
            "[Guardian] 当前实例已由伴生看门狗静默自愈拉起（第 {} 代，前序 PID={:?}，前序退出码={}），直接恢复托盘与轻量防断流守护",
            info.generation,
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
        win32_impl::init_session_and_spawn_companion(&log_dir, singleton_port, current_gen);
    }
}

/// 同步更新看门狗感知的「主进程是否处于后台托盘/轻量驻留模式」内核事件状态。
pub fn set_lightweight_mode_signal(in_lightweight: bool) {
    #[cfg(windows)]
    win32_impl::set_lightweight_mode_signal(in_lightweight);
}

/// 在用户显式点击退出或收到系统关机/注销信号的第一微秒通知看门狗与本地监督线程正常退场。
pub fn notify_intentional_exit() {
    #[cfg(windows)]
    win32_impl::notify_intentional_exit();
}

#[cfg(windows)]
mod win32_impl {
    use super::{
        Command, Duration, GUARD_RAPID_DEATH_WINDOW_SECS, GuardianArgs, MAX_RAPID_RESPAWN_GEN, Path, PathBuf,
        RESPAWN_SAFETY_DELAY_MS, STABLE_UPTIME_RESET_SECS, TcpListener, append_forensic_log,
        compute_guard_rebuild_backoff_ms, compute_next_generation, format_exit_code_diagnosis,
        is_installer_or_uninstaller_running, resolve_isolated_guardian_dir, sync_guardian_binary_to_dir,
    };
    use clash_verge_logging::{Type, logging};
    use std::{
        ffi::{OsStr, c_void},
        os::windows::{ffi::OsStrExt as _, io::AsRawHandle as _, process::CommandExt as _},
        process::Child,
        sync::{
            OnceLock,
            atomic::{AtomicBool, AtomicIsize, Ordering},
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
        fn GetCurrentProcessId() -> u32;
        fn GetModuleHandleW(lp_module_name: *const u16) -> Handle;
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
        pub(super) exit_event: isize,
        pub(super) lightweight_event: isize,
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

    fn resolve_guardian_executable(main_exe: &Path) -> PathBuf {
        let guard_dir = resolve_isolated_guardian_dir();
        match sync_guardian_binary_to_dir(main_exe, &guard_dir) {
            Ok(isolated) => isolated,
            Err(err) => {
                logging!(
                    warn,
                    Type::Setup,
                    "[Guardian] 同步物理隔离看门狗二进制失败，降级使用主程序路径 ({}): {}",
                    main_exe.display(),
                    err
                );
                main_exe.to_path_buf()
            }
        }
    }

    fn spawn_companion_child(
        guard_exe: &Path,
        main_exe: &Path,
        pid: u32,
        log_dir: &Path,
        singleton_port: u16,
        generation: u32,
    ) -> std::io::Result<Child> {
        let main_exe_str = main_exe.to_string_lossy().into_owned();
        let log_dir_str = log_dir.to_string_lossy().into_owned();
        let pid_str = pid.to_string();
        let port_str = singleton_port.to_string();
        let gen_str = generation.to_string();

        let build_cmd = |flags: u32| {
            let mut cmd = Command::new(guard_exe);
            cmd.args([
                "--guardian",
                "--parent-pid",
                &pid_str,
                "--target-exe",
                &main_exe_str,
                "--log-dir",
                &log_dir_str,
                "--singleton-port",
                &port_str,
                "--gen",
                &gen_str,
            ])
            .creation_flags(flags);
            cmd
        };

        // 优先尝试脱离父 Job Object，避免外部按 Job 清理时连带波及看门狗；若宿主禁止 Breakaway 则平滑回退
        build_cmd(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB)
            .spawn()
            .or_else(|_| build_cmd(CREATE_NO_WINDOW).spawn())
    }

    fn wait_for_companion_or_exit(child: &Child, exit_event: isize) -> u32 {
        let child_handle = child.as_raw_handle() as Handle;
        // SAFETY: child_handle 在 Child 存活期间有效，exit_event 为进程生命周期常驻事件句柄。
        unsafe {
            if exit_event == 0 {
                WaitForSingleObject(child_handle, INFINITE)
            } else {
                let handles = [child_handle, exit_event as Handle];
                WaitForMultipleObjects(2, handles.as_ptr(), 0, INFINITE)
            }
        }
    }

    fn run_companion_supervisor(
        main_exe: PathBuf,
        pid: u32,
        log_dir: PathBuf,
        singleton_port: u16,
        initial_gen: u32,
        exit_event: isize,
    ) {
        let session_boot = Instant::now();
        let mut rebuild_count: u32 = 0;
        let mut rapid_deaths: u32 = 0;

        loop {
            if is_exit_event_signaled(exit_event)
                || crate::core::handle::Handle::global().is_exiting()
                || is_system_shutting_down()
            {
                break;
            }

            let effective_gen = if session_boot.elapsed().as_secs() >= STABLE_UPTIME_RESET_SECS {
                0
            } else {
                initial_gen
            };
            let guard_exe = resolve_guardian_executable(&main_exe);

            let mut child =
                match spawn_companion_child(&guard_exe, &main_exe, pid, &log_dir, singleton_port, effective_gen) {
                    Ok(child) => child,
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
                        if rapid_deaths >= 5 || wait_exit_or_timeout(exit_event, retry_ms) {
                            break;
                        }
                        continue;
                    }
                };

            let guard_pid = child.id();
            let guard_spawned_at = Instant::now();
            if rebuild_count == 0 {
                logging!(
                    info,
                    Type::Setup,
                    "[Guardian] 伴生看门狗已就绪 (guardian_pid={}, parent_pid={}, gen={}, guard_exe={})",
                    guard_pid,
                    pid,
                    effective_gen,
                    guard_exe.display()
                );
            } else {
                logging!(
                    warn,
                    Type::System,
                    "[Guardian] 伴生看门狗已自动重建恢复 (guardian_pid={}, parent_pid={}, 第{}次重建)",
                    guard_pid,
                    pid,
                    rebuild_count
                );
            }

            let wait_res = wait_for_companion_or_exit(&child, exit_event);
            if wait_res != WAIT_OBJECT_0
                || is_exit_event_signaled(exit_event)
                || crate::core::handle::Handle::global().is_exiting()
                || is_system_shutting_down()
            {
                // 正常退出路径：直接释放 Child 句柄立即退场，绝不阻塞主进程退出
                break;
            }

            let exit_code = child.wait().ok().and_then(|s| s.code()).map_or(0, |c| c as u32);
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

            if wait_exit_or_timeout(exit_event, backoff_ms) {
                break;
            }
            if is_installer_or_uninstaller_running() {
                logging!(
                    info,
                    Type::System,
                    "[Guardian] 检测到安装或卸载程序正在运行，停止重建伴生看门狗"
                );
                break;
            }
            rebuild_count = rebuild_count.saturating_add(1);
        }
    }

    pub(super) fn init_session_and_spawn_companion(log_dir: &Path, singleton_port: u16, current_gen: u32) {
        let pid = unsafe { GetCurrentProcessId() };
        let session = GUARDIAN_SESSION.get_or_init(|| {
            let exit_name = to_wide_null(&exit_event_name(pid));
            let lw_name = to_wide_null(&lightweight_event_name(pid));
            let initial_lw = i32::from(crate::module::lightweight::is_in_lightweight_mode());
            // SAFETY: 创建当前会话专属的手动重置命名事件句柄，随进程全生命周期保留。
            unsafe {
                let exit_ev = CreateEventW(std::ptr::null(), 1, 0, exit_name.as_ptr());
                let lw_ev = CreateEventW(std::ptr::null(), 1, initial_lw, lw_name.as_ptr());
                GuardianSession {
                    exit_event: exit_ev as isize,
                    lightweight_event: lw_ev as isize,
                }
            }
        });

        if session.lightweight_event != 0 && crate::module::lightweight::is_in_lightweight_mode() {
            unsafe {
                let _ = SetEvent(session.lightweight_event as Handle);
            }
        }

        if GUARDIAN_SUPERVISOR_STARTED.swap(true, Ordering::AcqRel) {
            return;
        }

        let Ok(main_exe) = std::env::current_exe() else {
            GUARDIAN_SUPERVISOR_STARTED.store(false, Ordering::Release);
            return;
        };
        let log_dir_owned = log_dir.to_path_buf();
        let exit_event = session.exit_event;

        if let Err(err) = std::thread::Builder::new()
            .name("clash-mini-guard-sup".to_string())
            .spawn(move || {
                run_companion_supervisor(main_exe, pid, log_dir_owned, singleton_port, current_gen, exit_event);
            })
        {
            GUARDIAN_SUPERVISOR_STARTED.store(false, Ordering::Release);
            logging!(warn, Type::Setup, "[Guardian] 创建伴生看门狗监督线程失败: {}", err);
        }
    }

    pub(super) fn set_lightweight_mode_signal(in_lightweight: bool) {
        let Some(session) = GUARDIAN_SESSION.get() else {
            return;
        };
        if session.lightweight_event == 0 {
            return;
        }
        // SAFETY: lightweight_event 为有效的手动重置事件句柄。
        unsafe {
            if in_lightweight {
                let _ = SetEvent(session.lightweight_event as Handle);
            } else {
                let _ = ResetEvent(session.lightweight_event as Handle);
            }
        }
    }

    pub(super) fn notify_intentional_exit() {
        if let Some(session) = GUARDIAN_SESSION.get()
            && session.exit_event != 0
        {
            // SAFETY: exit_event 为有效的手动重置事件句柄。
            unsafe {
                let _ = SetEvent(session.exit_event as Handle);
            }
        }
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

    /// 阻塞等待父进程退出或主动退出事件，同时按需响应哨兵窗口消息（平时 0% CPU 内核态挂起）。
    fn wait_parent_with_message_pump(h_parent: Handle, h_exit: Handle) -> u32 {
        let handles = [h_parent, h_exit];
        let count: u32 = if h_exit.is_null() { 1 } else { 2 };
        loop {
            // SAFETY: handles 前 count 项均为有效内核同步句柄。
            let res = unsafe { MsgWaitForMultipleObjects(count, handles.as_ptr(), 0, INFINITE, QS_ALLINPUT) };
            if res == WAIT_OBJECT_0 + count {
                pump_pending_window_messages();
            } else {
                return res;
            }
        }
    }

    pub(super) fn run_guardian_loop(args: &GuardianArgs) {
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
                return;
            }

            let guard_sentinel = create_sentinel_window_pair("ClashMiniGuardHost", "Clash Mini Guard Host");
            pump_pending_window_messages();

            let h_exit = OpenEventW(SYNCHRONIZE | EVENT_MODIFY_STATE, 0, exit_name.as_ptr());
            let h_lw = OpenEventW(SYNCHRONIZE, 0, lw_name.as_ptr());

            let wait_result = wait_parent_with_message_pump(h_parent, h_exit);
            destroy_sentinel_window_pair(guard_sentinel);

            // 若正常退出事件已触发或系统正在关机，0ms 静默退场
            let intentional = !h_exit.is_null()
                && (wait_result == WAIT_OBJECT_0 + 1 || WaitForSingleObject(h_exit, 0) == WAIT_OBJECT_0);

            if intentional || wait_result != WAIT_OBJECT_0 || is_system_shutting_down() {
                if !h_lw.is_null() {
                    let _ = CloseHandle(h_lw);
                }
                if !h_exit.is_null() {
                    let _ = CloseHandle(h_exit);
                }
                let _ = CloseHandle(h_parent);
                return;
            }

            let mut exit_code: u32 = 0;
            let _ = GetExitCodeProcess(h_parent, &raw mut exit_code);
            let uptime_secs = query_process_uptime_secs(h_parent);
            let in_lightweight = !h_lw.is_null() && WaitForSingleObject(h_lw, 0) == WAIT_OBJECT_0;

            if !h_lw.is_null() {
                let _ = CloseHandle(h_lw);
            }
            if !h_exit.is_null() {
                let _ = CloseHandle(h_exit);
            }
            let _ = CloseHandle(h_parent);

            handle_unexpected_termination(args, exit_code, uptime_secs, in_lightweight);
        }
    }

    fn handle_unexpected_termination(args: &GuardianArgs, exit_code: u32, uptime_secs: u64, in_lightweight: bool) {
        let diag = format_exit_code_diagnosis(exit_code);
        append_forensic_log(
            &args.log_dir,
            "ERROR",
            &format!(
                "检测到主进程非正常终止 (PID={}, 存活={}s, 轻量/托盘驻留={}, ExitCode={})",
                args.parent_pid, uptime_secs, in_lightweight, diag
            ),
        );

        if !in_lightweight {
            return;
        }

        let Some(next_gen) = compute_next_generation(args.generation, uptime_secs) else {
            append_forensic_log(
                &args.log_dir,
                "ERROR",
                &format!(
                    "主进程在 {STABLE_UPTIME_RESET_SECS} 秒内连续异常终止已达 {MAX_RAPID_RESPAWN_GEN} 次上限，停止自动拉起以防止死循环"
                ),
            );
            return;
        };

        // 安全避让窗口：给 NSIS 安装器/卸载器或整组进程清理留出充足收尾时间
        std::thread::sleep(Duration::from_millis(RESPAWN_SAFETY_DELAY_MS));

        if is_system_shutting_down() {
            return;
        }

        if !args.target_exe.exists() {
            append_forensic_log(
                &args.log_dir,
                "INFO",
                &format!("目标主程序不存在 ({})，看门狗主动退出", args.target_exe.display()),
            );
            return;
        }

        if is_installer_or_uninstaller_running() {
            append_forensic_log(
                &args.log_dir,
                "INFO",
                "检测到安装或卸载程序正在运行，看门狗主动避让退出",
            );
            return;
        }

        // 若单例端口已被用户手动重启的新实例绑定，则主动避让
        if TcpListener::bind(("127.0.0.1", args.singleton_port)).is_err() {
            return;
        }

        let mut cmd = Command::new(&args.target_exe);
        if let Some(install_dir) = args.target_exe.parent().filter(|p| p.is_dir()) {
            cmd.current_dir(install_dir);
        }
        let spawn_res = cmd
            .args([
                "--guardian-respawn",
                "--guardian-gen",
                &next_gen.to_string(),
                "--prev-pid",
                &args.parent_pid.to_string(),
                "--prev-exit",
                &exit_code.to_string(),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();

        match spawn_res {
            Ok(respawned) => {
                append_forensic_log(
                    &args.log_dir,
                    "WARN",
                    &format!(
                        "伴生看门狗已完成静默托盘自愈拉起 (new_pid={}, gen={}, target={})",
                        respawned.id(),
                        next_gen,
                        args.target_exe.display()
                    ),
                );
            }
            Err(err) => {
                append_forensic_log(&args.log_dir, "ERROR", &format!("静默拉起主进程失败: {err}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GuardianArgs, ISOLATED_GUARDIAN_BINARY_NAME, MAX_RAPID_RESPAWN_GEN, RespawnInfo, STABLE_UPTIME_RESET_SECS,
        append_forensic_log, compute_guard_rebuild_backoff_ms, compute_next_generation, format_exit_code_diagnosis,
        parse_guardian_args_from_iter, parse_respawn_info_from_iter, sync_guardian_binary_to_dir,
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
            "--singleton-port",
            "33335",
            "--gen",
            "2",
        ];
        let parsed = parse_guardian_args_from_iter(args);
        assert_eq!(
            parsed,
            Some(GuardianArgs {
                parent_pid: 13712,
                target_exe: PathBuf::from(r"C:\Program Files\Clash Mini\clash-mini.exe"),
                log_dir: PathBuf::from(r"C:\Temp\logs"),
                singleton_port: 33335,
                generation: 2,
            })
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
            legacy_parsed.map(|p| p.target_exe),
            Some(PathBuf::from(r"C:\Program Files\Clash Mini\clash-mini.exe"))
        );
        assert_eq!(parse_guardian_args_from_iter(["clash-mini.exe", "--silent"]), None);
    }

    #[test]
    fn test_parse_respawn_info() {
        let args = [
            "clash-mini.exe",
            "--guardian-respawn",
            "--guardian-gen",
            "2",
            "--prev-pid",
            "9527",
            "--prev-exit",
            "99",
        ];
        assert_eq!(
            parse_respawn_info_from_iter(args),
            Some(RespawnInfo {
                generation: 2,
                prev_pid: Some(9527),
                prev_exit_code: Some(99),
            })
        );
        assert_eq!(parse_respawn_info_from_iter(["clash-mini.exe"]), None);
    }

    #[test]
    fn test_compute_next_generation_backoff_and_120s_reset() {
        assert_eq!(STABLE_UPTIME_RESET_SECS, 120);
        assert_eq!(compute_next_generation(0, 10), Some(1));
        assert_eq!(compute_next_generation(1, 50), Some(2));
        assert_eq!(compute_next_generation(2, 119), Some(MAX_RAPID_RESPAWN_GEN));
        assert_eq!(compute_next_generation(MAX_RAPID_RESPAWN_GEN, 119), None);
        // 稳定运行满 120 秒后再次被外部强杀，代数重置为 1（支持 7x24 小时无限次自愈）
        assert_eq!(
            compute_next_generation(MAX_RAPID_RESPAWN_GEN, STABLE_UPTIME_RESET_SECS),
            Some(1)
        );
        assert_eq!(
            compute_next_generation(MAX_RAPID_RESPAWN_GEN, STABLE_UPTIME_RESET_SECS + 3600),
            Some(1)
        );
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

            // 模拟升级后主二进制体积变化，应自动原子更新看门狗副本
            let _ = std::fs::write(&fake_main, b"MZ-v2-upgraded-binary-payload-longer");
            let updated = sync_guardian_binary_to_dir(&fake_main, &guard_dir);
            assert_eq!(updated.ok(), Some(guard_exe.clone()));
            assert_eq!(
                std::fs::read(&guard_exe).unwrap_or_default(),
                b"MZ-v2-upgraded-binary-payload-longer".to_vec()
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
    fn test_append_forensic_log_writes_latest_log() {
        let dir = std::env::temp_dir().join(format!("clash_mini_guardian_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        append_forensic_log(&dir, "ERROR", "unit test forensic message");
        let content = std::fs::read_to_string(dir.join("latest.log")).unwrap_or_default();
        assert!(content.contains("ERROR [System] [Guardian] unit test forensic message"));
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
