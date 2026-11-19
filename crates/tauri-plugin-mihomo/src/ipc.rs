use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::{Arc, Mutex as StdMutex, OnceLock},
    task::{Context, Poll},
    time::{Duration, Instant},
};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{
    client::conn::http1,
    rt::{Read, ReadBufCursor, Write},
};
use pin_project::pin_project;
use reqwest::RequestBuilder;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Semaphore, SemaphorePermit},
    time::timeout,
};
use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

use crate::{Error, Result};

#[pin_project(project = WrapStreamProj)]
pub enum WrapStream {
    NamedPipe(#[pin] NamedPipeClient),
}

impl WrapStream {
    pub async fn readable(&self) -> std::io::Result<()> {
        match self {
            WrapStream::NamedPipe(s) => s.readable().await,
        }
    }
    pub async fn writable(&self) -> std::io::Result<()> {
        match self {
            WrapStream::NamedPipe(s) => s.writable().await,
        }
    }
}

impl AsyncRead for WrapStream {
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.project() {
            WrapStreamProj::NamedPipe(s) => s.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for WrapStream {
    #[inline]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.project() {
            WrapStreamProj::NamedPipe(s) => s.poll_write(cx, buf),
        }
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            WrapStreamProj::NamedPipe(s) => s.poll_flush(cx),
        }
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            WrapStreamProj::NamedPipe(s) => s.poll_shutdown(cx),
        }
    }
}

impl Read for WrapStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, mut buf: ReadBufCursor<'_>) -> Poll<std::io::Result<()>> {
        let n = unsafe {
            let mut t_buf = tokio::io::ReadBuf::uninit(buf.as_mut());
            match tokio::io::AsyncRead::poll_read(self, cx, &mut t_buf) {
                Poll::Ready(Ok(())) => t_buf.filled().len(),
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        };
        unsafe {
            buf.advance(n);
        }
        Poll::Ready(Ok(()))
    }
}

impl Write for WrapStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        tokio::io::AsyncWrite::poll_write(self, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        tokio::io::AsyncWrite::poll_flush(self, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        tokio::io::AsyncWrite::poll_shutdown(self, cx)
    }
}

#[cfg(windows)]
fn wait_named_pipe_blocking(socket_path: &str, timeout_ms: u32) -> bool {
    use std::os::windows::ffi::OsStrExt as _;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn WaitNamedPipeW(lpNamedPipeName: *const u16, nTimeOut: u32) -> i32;
    }

    let wide: Vec<u16> = std::ffi::OsStr::new(socket_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `wide` is a valid NUL-terminated UTF-16 string living across the FFI call.
    unsafe { WaitNamedPipeW(wide.as_ptr(), timeout_ms) != 0 }
}

pub async fn connect_to_socket(socket_path: &str) -> Result<WrapStream> {
    let mut max_retry_count = 3;
    // WARNING: DO NOT decrease busy_retry_count!
    // Under high concurrency (e.g. 36 parallel speed test workers), Named Pipes often return ERROR_PIPE_BUSY.
    // A retry count of 24 with WaitNamedPipeW (<= 100ms per step, <= 2.4s total < 5.0s request timeout)
    // wakes immediately when an instance frees up while avoiding outer timeout deadlock (BUG-330 / BUG-351).
    let mut busy_retry_count = 24;
    const PIPE_WAIT_MS: u32 = 100;
    const FALLBACK_RETRY_DELAY: Duration = Duration::from_millis(80);

    let client = loop {
        match ClientOptions::new().open(socket_path) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                if busy_retry_count == 0 {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Named pipe busy timeout exceeded",
                    )));
                }
                busy_retry_count -= 1;
                let path_owned = socket_path.to_string();
                let waited = tokio::task::spawn_blocking(move || wait_named_pipe_blocking(&path_owned, PIPE_WAIT_MS))
                    .await
                    .unwrap_or(false);
                if !waited {
                    tokio::time::sleep(FALLBACK_RETRY_DELAY).await;
                }
                continue;
            }
            Err(e) => {
                log::warn!("failed to connect to named pipe: {socket_path}, {e}");
                if max_retry_count == 0 {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Failed to connect to named pipe: {socket_path}, {e}"),
                    )));
                }
                max_retry_count -= 1;
            }
        }
        tokio::time::sleep(FALLBACK_RETRY_DELAY).await;
    };
    Ok(WrapStream::NamedPipe(client))
}

// ----------------------------------------------------------------
//                       Connection Pool
// ----------------------------------------------------------------

// 连接池配置
#[derive(Debug, Clone)]
pub struct IpcPoolConfig {
    /// 最小连接数, 默认 `3`
    pub min_connections: usize,
    /// 最大连接数, 默认 `10`
    pub max_connections: usize,
    /// 空闲超时时间, 默认 `60s`
    pub idle_timeout: Duration,
    /// 健康检查间隔, 默认 `60s`
    pub health_check_interval: Duration,
    /// 拒绝策略, 默认 `New` （无需等待连接池可用，直接创建新的 IPC 连接）
    pub reject_policy: RejectPolicy,
}

impl Default for IpcPoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 3,
            max_connections: 20,
            idle_timeout: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(60),
            reject_policy: RejectPolicy::New,
        }
    }
}

pub struct IpcPoolConfigBuilder {
    min_connections: usize,
    max_connections: usize,
    idle_timeout: Duration,
    health_check_interval: Duration,
    reject_policy: RejectPolicy,
}

impl Default for IpcPoolConfigBuilder {
    fn default() -> Self {
        Self {
            min_connections: 3,
            max_connections: 20,
            idle_timeout: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(60),
            reject_policy: RejectPolicy::New,
        }
    }
}

impl IpcPoolConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_connections(mut self, min_connections: usize) -> Self {
        self.min_connections = min_connections;
        self
    }

    pub fn max_connections(mut self, max_connections: usize) -> Self {
        self.max_connections = max_connections;
        self
    }

    pub fn idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }

    pub fn health_check_interval(mut self, health_check_interval: Duration) -> Self {
        self.health_check_interval = health_check_interval;
        self
    }

    pub fn reject_policy(mut self, reject_policy: RejectPolicy) -> Self {
        self.reject_policy = reject_policy;
        self
    }

    pub fn build(self) -> IpcPoolConfig {
        IpcPoolConfig {
            min_connections: self.min_connections,
            max_connections: self.max_connections,
            idle_timeout: self.idle_timeout,
            health_check_interval: self.health_check_interval,
            reject_policy: self.reject_policy,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub enum RejectPolicy {
    #[default]
    New, // 无需等待连接池的连接可用，直接创建新的 IPC 连接
    Reject,            // 连接池的连接不可用时，直接拒绝
    Timeout(Duration), // 等待连接池的连接可用的超时时间
    Wait,              // 一直等待连接池的连接可用
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PoolPermit<'a> {
    Semaphore(SemaphorePermit<'a>),
    Bypass,
}

// IPC 连接包装器：绑定创建它的 Tokio 运行时驱动句柄，丢弃时立即中止驱动释放底层命名管道句柄（BUG-406）
struct IpcConnection {
    sender: http1::SendRequest<Full<Bytes>>,
    last_used: Instant,
    driver_handle: tokio::task::JoinHandle<()>,
}

impl IpcConnection {
    #[inline]
    fn new(sender: http1::SendRequest<Full<Bytes>>, driver_handle: tokio::task::JoinHandle<()>) -> Self {
        Self {
            sender,
            last_used: Instant::now(),
            driver_handle,
        }
    }

    #[inline]
    fn is_valid(&self) -> bool {
        self.sender.is_ready() && !self.driver_handle.is_finished()
    }
}

impl Drop for IpcConnection {
    fn drop(&mut self) {
        self.driver_handle.abort();
    }
}

// IPC 连接池：按 tokio::runtime::Id 物理分桶隔离，杜绝 DAEMON_RUNTIME 与 tauri::async_runtime 跨 IOCP 复用
#[derive(Clone)]
pub struct IpcConnectionPool {
    connections: Arc<StdMutex<HashMap<tokio::runtime::Id, VecDeque<IpcConnection>>>>,
    semaphore: Arc<Semaphore>,
    config: IpcPoolConfig,
}

static CONNECTION_POOL: OnceLock<IpcConnectionPool> = OnceLock::new();

impl IpcConnectionPool {
    #[inline]
    fn new(config: IpcPoolConfig) -> Self {
        let pool = IpcConnectionPool {
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
            config,
            connections: Arc::new(StdMutex::new(HashMap::new())),
        };
        // 启动清理空闲连接的任务线程
        pool.start_clear_idle_conns_task();
        pool
    }

    /// 初始化全局实例
    #[inline]
    pub fn init(config: IpcPoolConfig) -> Result<()> {
        CONNECTION_POOL
            .set(IpcConnectionPool::new(config))
            .map_err(|_| Error::ConnectionPoolInitFailed)
    }

    #[inline]
    pub fn global() -> Result<&'static Self> {
        CONNECTION_POOL.get().ok_or(Error::ConnectionPoolNotInitialized)
    }

    /// 启动清理空闲连接的任务线程
    #[inline]
    fn start_clear_idle_conns_task(&self) {
        let pool = self.clone();

        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(pool.config.health_check_interval);
            loop {
                interval.tick().await;
                pool.cleanup_idle_connections();
            }
        });
    }

    // 清理空闲连接（锁内仅做状态转移，Drop / abort 统一延后到锁外执行）
    #[inline]
    fn cleanup_idle_connections(&self) {
        let now = Instant::now();
        let min_to_keep = self.config.min_connections;

        let mut total_checked = 0usize;
        let mut kept = 0usize;
        let mut evicted = Vec::new();

        {
            let mut buckets = self.connections.lock().unwrap_or_else(|e| e.into_inner());
            buckets.retain(|_, queue| {
                let mut next_queue = VecDeque::with_capacity(queue.len());
                let mut bucket_kept = 0usize;
                while let Some(conn) = queue.pop_front() {
                    total_checked += 1;
                    if conn.is_valid() {
                        let is_idle_timeout = now.duration_since(conn.last_used) > self.config.idle_timeout;
                        if bucket_kept < min_to_keep || !is_idle_timeout {
                            next_queue.push_back(conn);
                            bucket_kept += 1;
                            kept += 1;
                        } else {
                            evicted.push(conn);
                        }
                    } else {
                        log::debug!("cleanup: dropping invalid connection");
                        evicted.push(conn);
                    }
                }
                *queue = next_queue;
                !queue.is_empty()
            });
        }

        drop(evicted);
        log::debug!("Cleanup done: checked {}, kept {}", total_checked, kept);
    }

    #[inline]
    fn pop_valid_connection(&self, runtime_id: tokio::runtime::Id) -> Option<IpcConnection> {
        let mut stale = Vec::new();
        let mut found = None;
        {
            let mut buckets = self.connections.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(queue) = buckets.get_mut(&runtime_id) {
                while let Some(conn) = queue.pop_back() {
                    if conn.is_valid() {
                        found = Some(conn);
                        break;
                    }
                    stale.push(conn);
                }
                if queue.is_empty() {
                    buckets.remove(&runtime_id);
                }
            }
        }
        drop(stale);
        found
    }

    #[inline]
    fn return_connection(&self, runtime_id: tokio::runtime::Id, mut conn: IpcConnection) {
        conn.last_used = Instant::now();
        let mut buckets = self.connections.lock().unwrap_or_else(|e| e.into_inner());
        buckets.entry(runtime_id).or_default().push_back(conn);
    }

    #[inline]
    async fn get_connection<'a>(&'a self, socket_path: &str) -> Result<(IpcConnection, bool, PoolPermit<'a>)> {
        log::debug!("get connection from pool");
        // 确保获取 pool permit
        let permit = self.acquire_permit().await?;
        // 开始创建或复用连接（返回 reused 标记以支持半开连接透明重建重试，BUG-329 / BUG-351）
        let (conn, reused) = self.acquire_or_create_connection(socket_path).await?;
        Ok((conn, reused, permit))
    }

    async fn acquire_permit<'a>(&'a self) -> Result<PoolPermit<'a>> {
        log::debug!("acquire permit");
        match self.semaphore.try_acquire() {
            Ok(permit) => Ok(PoolPermit::Semaphore(permit)),
            Err(_) => match self.config.reject_policy {
                RejectPolicy::New => {
                    log::debug!("max permit has acquire, bypass semaphore");
                    Ok(PoolPermit::Bypass)
                }
                RejectPolicy::Reject => Err(Error::ConnectionPoolFull),
                RejectPolicy::Timeout(timeout_duration) => {
                    let acquire_future = self.semaphore.acquire();
                    match timeout(timeout_duration, acquire_future).await {
                        Ok(Ok(permit)) => Ok(PoolPermit::Semaphore(permit)),
                        Ok(Err(_)) => Err(Error::ConnectionPoolFull),
                        Err(e) => Err(Error::Timeout(e)),
                    }
                }
                RejectPolicy::Wait => {
                    let acquire_future = self.semaphore.acquire().await;
                    match acquire_future {
                        Ok(permit) => Ok(PoolPermit::Semaphore(permit)),
                        Err(_) => Err(Error::ConnectionPoolFull),
                    }
                }
            },
        }
    }

    async fn acquire_or_create_connection(&self, socket_path: &str) -> Result<(IpcConnection, bool)> {
        let runtime_id = tokio::runtime::Handle::current().id();
        if let Some(conn) = self.pop_valid_connection(runtime_id) {
            log::debug!("Reusing connection from pool for runtime {:?}", runtime_id);
            return Ok((conn, true));
        }

        log::trace!("Pool empty for runtime {:?}, creating new connection", runtime_id);
        let conn = Self::create_connection(socket_path).await?;
        Ok((conn, false))
    }

    async fn create_connection(socket_path: &str) -> Result<IpcConnection> {
        log::trace!(
            "creating connection, available permits: {}",
            Self::global()?.semaphore.available_permits()
        );
        let stream = match connect_to_socket(socket_path).await {
            Ok(stream) => stream,
            Err(e) => return Err(Error::ConnectionFailed(e.to_string())),
        };
        let (sender, conn_driver) = http1::handshake(stream)
            .await
            .map_err(|e| Error::HttpParseError(e.to_string()))?;

        // 将 conn_driver 严格绑定到打开 NamedPipeClient 的同一 Tokio 运行时与 IOCP 上（BUG-406）
        let driver_handle = tokio::spawn(async move {
            if let Err(err) = conn_driver.await {
                log::debug!("IPC Connection Error: {:?}", err);
            }
        });

        Ok(IpcConnection::new(sender, driver_handle))
    }

    pub fn clear_pool(&self) {
        let drained = {
            let mut buckets = self.connections.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *buckets)
        };
        drop(drained);
        log::debug!("IpcConnectionPool cleared");
    }
}

impl Drop for IpcConnectionPool {
    fn drop(&mut self) {
        log::debug!("IpcConnectionPool is being dropped");
        self.clear_pool();
    }
}

pub trait LocalSocket {
    async fn send_by_local_socket(self, socket_path: &str) -> Result<reqwest::Response>;
}

impl LocalSocket for RequestBuilder {
    async fn send_by_local_socket(self, socket_path: &str) -> Result<reqwest::Response> {
        let reqwest_req = self.build()?;
        let timeout_dur = reqwest_req.timeout();

        let method = reqwest_req.method().clone();
        let url_str = reqwest_req.url().as_str().to_string();
        let headers = reqwest_req.headers().clone();

        let body_bytes = if let Some(body) = reqwest_req.body() {
            body.as_bytes().map(Bytes::copy_from_slice).unwrap_or_else(Bytes::new)
        } else {
            Bytes::new()
        };

        let make_hyper_req =
            |m: reqwest::Method, u: &str, h: &http::HeaderMap, b: &Bytes| -> Result<http::Request<Full<Bytes>>> {
                let mut builder = http::Request::builder().method(m).uri(u);
                if let Some(hdrs) = builder.headers_mut() {
                    *hdrs = h.clone();
                }
                Ok(builder.body(Full::new(b.clone()))?)
            };

        let process = async move {
            let pool = IpcConnectionPool::global()?;
            let (mut conn, reused, _permit) = pool.get_connection(socket_path).await?;
            let first_req = make_hyper_req(method.clone(), &url_str, &headers, &body_bytes)?;

            let is_idempotent_read = matches!(method, reqwest::Method::GET | reqwest::Method::HEAD);
            let hyper_res = match conn.sender.send_request(first_req).await {
                Ok(res) => res,
                Err(first_err) if reused && is_idempotent_read => {
                    // 池中复用的空闲命名管道可能在对端热重载或空闲回收时已半关闭（is_ready 尚未感知）；
                    // 仅对 GET / HEAD 幂等只读请求自动丢弃旧连接并新建物理管道透明重发 1 次，
                    // 非幂等写请求（PUT/POST/PATCH/DELETE）严禁传输层盲目重发以免内核重复执行（BUG-329 / BUG-351）。
                    log::debug!(
                        "Pooled IPC connection closed on idempotent {method} ({first_err}), retrying once on fresh pipe"
                    );
                    conn = IpcConnectionPool::create_connection(socket_path).await?;
                    let retry_req = make_hyper_req(method, &url_str, &headers, &body_bytes)?;
                    conn.sender
                        .send_request(retry_req)
                        .await
                        .map_err(|e| Error::HttpParseError(e.to_string()))?
                }
                Err(e) => return Err(Error::HttpParseError(e.to_string())),
            };

            let (res_parts, res_body) = hyper_res.into_parts();
            let collected_body = res_body
                .collect()
                .await
                .map_err(|e| Error::HttpParseError(e.to_string()))?
                .to_bytes();

            let final_res = http::Response::from_parts(res_parts, collected_body);

            // 返还连接回当前运行时所属桶
            pool.return_connection(tokio::runtime::Handle::current().id(), conn);

            Ok(reqwest::Response::from(final_res))
        };

        match timeout_dur {
            Some(d) => timeout(*d, process).await?,
            None => process.await,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt as _;

    #[pin_project]
    struct DuplexHyperIo(#[pin] tokio::io::DuplexStream);

    impl Read for DuplexHyperIo {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            mut buf: ReadBufCursor<'_>,
        ) -> Poll<std::io::Result<()>> {
            let n = unsafe {
                let mut t_buf = tokio::io::ReadBuf::uninit(buf.as_mut());
                match AsyncRead::poll_read(self.project().0, cx, &mut t_buf) {
                    Poll::Ready(Ok(())) => t_buf.filled().len(),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            };
            unsafe {
                buf.advance(n);
            }
            Poll::Ready(Ok(()))
        }
    }

    impl Write for DuplexHyperIo {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
            AsyncWrite::poll_write(self.project().0, cx, buf)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            AsyncWrite::poll_flush(self.project().0, cx)
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            AsyncWrite::poll_shutdown(self.project().0, cx)
        }
    }

    fn make_test_pool() -> IpcConnectionPool {
        let config = IpcPoolConfig::default();
        IpcConnectionPool {
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
            config,
            connections: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    #[test]
    fn ipc_pool_isolates_buckets_by_runtime_and_drop_aborts_driver() {
        let rt_a = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build runtime A");
        let rt_b = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build runtime B");

        let pool = make_test_pool();

        // 1. 在 Runtime A 创建连接并入池，验证 Runtime B 绝不能借走 Runtime A 的管道句柄
        let (mut peer_a_for_isolation, id_a) = rt_a.block_on(async {
            let (client_stream, server_stream) = tokio::io::duplex(256);
            let (mut sender, conn_driver) = http1::handshake(DuplexHyperIo(client_stream))
                .await
                .expect("handshake A");
            let driver_handle = tokio::spawn(async move {
                let _ = conn_driver.await;
            });
            sender.ready().await.expect("sender A ready");
            let id = tokio::runtime::Handle::current().id();
            pool.return_connection(id, IpcConnection::new(sender, driver_handle));
            (server_stream, id)
        });

        rt_b.block_on(async {
            let id_b = tokio::runtime::Handle::current().id();
            assert_ne!(id_a, id_b);
            assert!(
                pool.pop_valid_connection(id_b).is_none(),
                "Runtime B 不得跨桶复用 Runtime A 的命名管道连接"
            );
        });

        let popped_a = rt_a.block_on(async {
            let id = tokio::runtime::Handle::current().id();
            pool.pop_valid_connection(id)
        });
        assert!(popped_a.is_some(), "Runtime A 应能命中自己桶中的连接");

        // 2. Drop IpcConnection（等价于超时取消）必须立刻 abort driver_handle 并关闭对端句柄
        rt_a.block_on(async move {
            drop(popped_a);
            let mut buf = [0u8; 8];
            let read_res = timeout(Duration::from_millis(200), peer_a_for_isolation.read(&mut buf))
                .await
                .expect("peer should observe EOF promptly after driver abort")
                .expect("read succeeds with EOF");
            assert_eq!(read_res, 0, "Drop 后驱动必须终止并释放底层流句柄");
        });

        // 3. clear_pool 也必须立刻终止并释放所有空闲连接
        let mut peer_b = rt_b.block_on(async {
            let (client_stream, server_stream) = tokio::io::duplex(256);
            let (mut sender, conn_driver) = http1::handshake(DuplexHyperIo(client_stream))
                .await
                .expect("handshake B");
            let driver_handle = tokio::spawn(async move {
                let _ = conn_driver.await;
            });
            sender.ready().await.expect("sender B ready");
            pool.return_connection(
                tokio::runtime::Handle::current().id(),
                IpcConnection::new(sender, driver_handle),
            );
            server_stream
        });

        pool.clear_pool();

        rt_b.block_on(async move {
            let mut buf = [0u8; 8];
            let read_res = timeout(Duration::from_millis(200), peer_b.read(&mut buf))
                .await
                .expect("peer B should observe EOF after clear_pool")
                .expect("read succeeds with EOF");
            assert_eq!(read_res, 0, "clear_pool 必须立刻 abort 空闲驱动并关闭句柄");
        });
    }
}
