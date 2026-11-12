// ─── AsyncHandler 设计说明 ───────────────────────────────────────────────────
//
// 当前设计决策：
// 1. spawn() 的 Future 输出为 ()，任务内部自行处理错误（通过 logging! 宏记录）
// 2. 不统一 abort on panic，因为：
//    - 大部分后台任务（日志清理、通知刷新等）失败不应导致应用崩溃
//    - 任务失败通过 logging_error! 宏统一记录，便于排查但不影响主流程
// 3. 对于关键路径任务（如配置保存），调用方应自行 join 并处理结果
//
// 未来改进方向：
// - 可添加 spawn_critical() 用于关键任务，失败时触发全局错误处理
// - 可添加任务监控面板，统计失败任务数量
// - 对于可能 panic 的任务，考虑使用 catch_unwind 隔离
// ──────────────────────────────────────────────────────────────────────────────

use std::future::Future;
use tauri::{async_runtime, async_runtime::JoinHandle};

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

    /// 生成带错误日志的后台任务
    /// 任务返回 Result，错误时自动通过 logging_error! 记录
    #[inline]
    #[track_caller]
    pub fn spawn_with_error_log<F, Fut>(log_type: clash_verge_logging::Type, f: F) -> JoinHandle<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        async_runtime::spawn(async move {
            let result = f().await;
            if let Err(e) = result {
                clash_verge_logging::logging!(error, log_type, "{}", e);
            }
        })
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

    /// 生成具备 Panic 隔离与退避自恢复能力的长期守护后台任务（BUG-353）
    /// 若内部 Future 正常退出则结束；若发生意外 Panic 则记录完整载荷并在应用未退出时经 2s 冷却自动重启守护。
    #[track_caller]
    pub fn spawn_supervised<F, Fut>(
        task_name: &'static str,
        log_type: clash_verge_logging::Type,
        mut factory: F,
    ) -> JoinHandle<()>
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        use futures::FutureExt as _;
        async_runtime::spawn(async move {
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
        })
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
