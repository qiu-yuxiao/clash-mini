use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::Duration,
};

use clash_verge_logging::{Type, logging};

/// 短时间（< 60s）内连续异常终止的最大自动拉起代数，防止真实启动级故障引发无限拉起死循环。
pub const MAX_RAPID_RESPAWN_GEN: u32 = 3;

/// 主进程稳定存活超过该秒数后，若再遭外部强杀，视为独立事件，拉起代数重置为 1。
pub const STABLE_UPTIME_RESET_SECS: u64 = 60;

/// 检测到主进程在轻量模式下被外部强杀后，拉起前的安全避让等待毫秒数（避让安装/卸载器与整组清理）。
const RESPAWN_SAFETY_DELAY_MS: u64 = 1500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardianArgs {
    pub parent_pid: u32,
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

    Some(GuardianArgs {
        parent_pid: parent_pid?,
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

/// 在主进程 UI 主线程创建常驻零 WebView 哨兵宿主窗口对（防 360 加速球 `DoOptmGroupResidual` 无窗口残留误杀）。
pub fn init_resident_sentinel_window() {
    #[cfg(windows)]
    win32_impl::init_resident_sentinel_window();
}

/// 在主进程初始化完成后创建同步内核事件并拉起超轻量 `--guardian` 伴生看门狗进程。
pub fn start_guardian_companion() {
    if cfg!(test) || std::env::var("CLASH_MINI_DISABLE_GUARDIAN").as_deref() == Ok("1") {
        return;
    }

    if let Some(info) = parse_respawn_info_from_iter(std::env::args()) {
        let prev_exit_desc = info
            .prev_exit_code
            .map(format_exit_code_diagnosis)
            .unwrap_or_else(|| "未知".to_string());
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

/// 同步更新看门狗感知的「主进程是否处于轻量模式」内核事件状态。
pub fn set_lightweight_mode_signal(in_lightweight: bool) {
    #[cfg(windows)]
    win32_impl::set_lightweight_mode_signal(in_lightweight);
}

/// 在用户显式点击退出或收到系统关机/注销信号的第一微秒通知看门狗正常退场。
pub fn notify_intentional_exit() {
    #[cfg(windows)]
    win32_impl::notify_intentional_exit();
}

#[cfg(windows)]
mod win32_impl {
    use super::{
        Command, Duration, GuardianArgs, Path, RESPAWN_SAFETY_DELAY_MS, TcpListener, append_forensic_log,
        compute_next_generation, format_exit_code_diagnosis, is_installer_or_uninstaller_running,
    };
    use clash_verge_logging::{Type, logging};
    use std::{
        ffi::{OsStr, c_void},
        os::windows::{ffi::OsStrExt as _, process::CommandExt as _},
        sync::{
            OnceLock,
            atomic::{AtomicIsize, Ordering},
        },
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
    const SM_SHUTTINGDOWN: i32 = 0x2000;

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
        fn GetSystemMetrics(n_index: i32) -> i32;
    }

    pub(super) struct GuardianSession {
        pub(super) exit_event: isize,
        pub(super) lightweight_event: isize,
    }

    static GUARDIAN_SESSION: OnceLock<GuardianSession> = OnceLock::new();
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
        // SAFETY: 标准默认窗口过程，转发所有消息以保证 IsHungAppWindow 恒为 FALSE。
        unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
    }

    /// 创建一对原生零渲染宿主窗口（隐藏 Owner + 屏外全透明穿透 `WS_POPUP | WS_VISIBLE` 归属窗口），返回 `(owner, sentinel)`。
    pub(super) fn create_sentinel_window_pair(class_name_str: &str) -> Option<(Hwnd, Hwnd)> {
        let class_name = to_wide_null(class_name_str);
        let owner_title = to_wide_null("ClashMiniResidentOwner");
        let sentinel_title = to_wide_null("Clash Mini");

        // SAFETY: 在当前线程注册自定义轻量窗口类并创建屏外 1x1 透明归属窗口对。
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
                -32000,
                -32000,
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

            let _ = SetLayeredWindowAttributes(sentinel, 0, 0, LWA_ALPHA);
            Some((owner, sentinel))
        }
    }

    pub(super) fn init_resident_sentinel_window() {
        if SENTINEL_VISIBLE_HWND.load(Ordering::Acquire) != 0 {
            return;
        }
        if let Some((owner, sentinel)) = create_sentinel_window_pair("ClashMiniResidentHost") {
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

        // 若在初始化完成前轻量状态已变更，同步对齐一次事件位
        if session.lightweight_event != 0 && crate::module::lightweight::is_in_lightweight_mode() {
            unsafe {
                let _ = SetEvent(session.lightweight_event as Handle);
            }
        }

        let Ok(exe_path) = std::env::current_exe() else {
            return;
        };
        let log_dir_str = log_dir.to_string_lossy().into_owned();

        let build_cmd = |flags: u32| {
            let mut cmd = Command::new(&exe_path);
            cmd.args([
                "--guardian",
                "--parent-pid",
                &pid.to_string(),
                "--log-dir",
                &log_dir_str,
                "--singleton-port",
                &singleton_port.to_string(),
                "--gen",
                &current_gen.to_string(),
            ])
            .creation_flags(flags);
            cmd
        };

        // 优先尝试脱离父 Job Object，避免外部按 Job 清理时连带波及看门狗；若宿主环境禁止 Breakaway 则平滑回退
        let spawned = build_cmd(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB)
            .spawn()
            .or_else(|_| build_cmd(CREATE_NO_WINDOW).spawn());

        match spawned {
            Ok(child) => {
                logging!(
                    info,
                    Type::Setup,
                    "[Guardian] 伴生看门狗已就绪 (guardian_pid={}, parent_pid={}, gen={})",
                    child.id(),
                    pid,
                    current_gen
                );
            }
            Err(err) => {
                logging!(warn, Type::Setup, "[Guardian] 启动伴生看门狗失败: {}", err);
            }
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
        if let Some(session) = GUARDIAN_SESSION.get() {
            if session.exit_event != 0 {
                // SAFETY: exit_event 为有效的手动重置事件句柄。
                unsafe {
                    let _ = SetEvent(session.exit_event as Handle);
                }
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

    pub(super) fn run_guardian_loop(args: &GuardianArgs) {
        let exit_name = to_wide_null(&exit_event_name(args.parent_pid));
        let lw_name = to_wide_null(&lightweight_event_name(args.parent_pid));

        // SAFETY: 打开父进程同步句柄与两个命名同步事件句柄，全程使用完后显式 CloseHandle。
        unsafe {
            let h_parent = OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                args.parent_pid,
            );
            if h_parent.is_null() {
                return;
            }

            let h_exit = OpenEventW(SYNCHRONIZE | EVENT_MODIFY_STATE, 0, exit_name.as_ptr());
            let h_lw = OpenEventW(SYNCHRONIZE, 0, lw_name.as_ptr());

            let wait_result = if h_exit.is_null() {
                WaitForSingleObject(h_parent, INFINITE)
            } else {
                let handles = [h_parent, h_exit];
                WaitForMultipleObjects(2, handles.as_ptr(), 0, INFINITE)
            };

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
                "检测到主进程非正常终止 (PID={}, 存活={}s, 轻量模式={}, ExitCode={})",
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
                "主进程在 60 秒内连续异常终止已达 3 次上限，停止自动拉起以防止死循环",
            );
            return;
        };

        // 安全避让窗口：给 NSIS 安装器/卸载器或整组进程清理留出充足收尾时间
        std::thread::sleep(Duration::from_millis(RESPAWN_SAFETY_DELAY_MS));

        if is_system_shutting_down() {
            return;
        }

        let Ok(exe_path) = std::env::current_exe() else {
            return;
        };
        if !exe_path.exists() {
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

        let spawn_res = Command::new(&exe_path)
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

        if let Err(err) = spawn_res {
            append_forensic_log(&args.log_dir, "ERROR", &format!("静默拉起主进程失败: {err}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GuardianArgs, MAX_RAPID_RESPAWN_GEN, RespawnInfo, STABLE_UPTIME_RESET_SECS, append_forensic_log,
        compute_next_generation, format_exit_code_diagnosis, parse_guardian_args_from_iter,
        parse_respawn_info_from_iter,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_guardian_args() {
        let args = [
            "clash-mini.exe",
            "--guardian",
            "--parent-pid",
            "13712",
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
                log_dir: PathBuf::from(r"C:\Temp\logs"),
                singleton_port: 33335,
                generation: 2,
            })
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
    fn test_compute_next_generation_backoff_and_reset() {
        assert_eq!(compute_next_generation(0, 10), Some(1));
        assert_eq!(compute_next_generation(1, 20), Some(2));
        assert_eq!(compute_next_generation(2, 30), Some(MAX_RAPID_RESPAWN_GEN));
        assert_eq!(compute_next_generation(MAX_RAPID_RESPAWN_GEN, 5), None);
        // 稳定运行满 60 秒后再次被外部强杀，代数重置为 1
        assert_eq!(
            compute_next_generation(MAX_RAPID_RESPAWN_GEN, STABLE_UPTIME_RESET_SECS + 15),
            Some(1)
        );
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
        unsafe extern "system" {
            fn IsWindowVisible(h_wnd: *mut std::ffi::c_void) -> i32;
            fn GetWindowLongPtrW(h_wnd: *mut std::ffi::c_void, n_index: i32) -> isize;
            fn DestroyWindow(h_wnd: *mut std::ffi::c_void) -> i32;
        }
        const GWL_EXSTYLE: i32 = -20;
        const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
        const WS_EX_TOPMOST: isize = 0x0000_0008;
        const WS_EX_APPWINDOW: isize = 0x0004_0000;

        let pair = super::win32_impl::create_sentinel_window_pair("ClashMiniResidentHostTest");
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
                let _ = DestroyWindow(sentinel);
                let _ = DestroyWindow(owner);
            }
        }
    }
}
