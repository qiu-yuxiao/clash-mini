use std::future::Future;
use std::sync::OnceLock;
use tauri::{async_runtime, async_runtime::JoinHandle};

/// 物理隔离的后台守护 Tokio 运行时（BUG-394）：
/// 专用于运行「后台节点健康监测与故障自愈 (`module/monitor/`)」及「单实例/PAC 监听 (`server.rs`)」。
/// 彻底脱离 Tauri 默认 async_runtime 工作池，即便极端高负载下 Win32 主线程 / WebView2 创建阻塞
/// 引发 UI/命令工作池满载，后台断流自愈与单实例端口响应依然保持 100% 调度畅通。
static DAEMON_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn daemon_runtime() -> &'static tokio::runtime::Runtime {
    DAEMON_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("clash-mini-daemon")
            .enable_all()
            .build()
            .or_else(|e| {
                clash_verge_logging::logging!(
                    warn,
                    clash_verge_logging::Type::Core,
                    "[DaemonRuntime] 创建双线程守护运行时失败，降级单线程守护池: {e}"
                );
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .thread_name("clash-mini-daemon")
                    .enable_all()
                    .build()
            })
            .unwrap_or_else(|e| {
                #[allow(clippy::panic)]
                {
                    panic!("failed to initialize clash-mini daemon tokio runtime: {e}");
                }
            })
    })
}

async fn run_supervised_loop<F, Fut>(task_name: &'static str, log_type: clash_verge_logging::Type, mut factory: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    use futures::FutureExt as _;
    loop {
        if crate::core::handle::Handle::global().is_exiting() {
            break;
        }
        let fut = std::panic::AssertUnwindSafe(factory());
        match fut.catch_unwind().await {
            Ok(()) => break,
            Err(panic_payload) => {
                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic payload".to_string()
                };
                clash_verge_logging::logging!(
                    error,
                    log_type,
                    "[SupervisedTask:{task_name}] 捕获未预期 Panic: {msg}，2s 后尝试自愈重启"
                );
                if crate::core::handle::Handle::global().is_exiting() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

pub struct AsyncHandler;

impl AsyncHandler {
    #[inline]
    #[track_caller]
    pub fn spawn<F, Fut>(f: F) -> JoinHandle<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        async_runtime::spawn(f())
    }

    #[inline]
    #[track_caller]
    pub fn spawn_blocking<T, F>(f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        async_runtime::spawn_blocking(f)
    }

    /// 在物理隔离的专属守护 Tokio 运行时上派发异步任务
    #[inline]
    #[track_caller]
    pub fn spawn_daemon<F, Fut>(f: F) -> tokio::task::JoinHandle<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        daemon_runtime().spawn(f())
    }

    /// 在物理隔离的专属守护 Tokio 运行时上派发带 Panic 自愈监督的常驻守护任务
    #[track_caller]
    pub fn spawn_daemon_supervised<F, Fut>(
        task_name: &'static str,
        log_type: clash_verge_logging::Type,
        factory: F,
    ) -> tokio::task::JoinHandle<()>
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        daemon_runtime().spawn(run_supervised_loop(task_name, log_type, factory))
    }

    #[inline]
    #[track_caller]
    pub fn block_on<Fut>(fut: Fut) -> Fut::Output
    where
        Fut: Future + Send + 'static,
    {
        async_runtime::block_on(fut)
    }
}
