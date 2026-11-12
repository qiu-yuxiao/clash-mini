use std::{
    pin::Pin,
    sync::{Arc, OnceLock},
    task::{Context, Poll},
    time::{Duration, Instant},
};

use bytes::Bytes;
use crossbeam::queue::SegQueue;
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
                let waited =
                    tauri::async_runtime::spawn_blocking(move || wait_named_pipe_blocking(&path_owned, PIPE_WAIT_MS))
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

// IPC 连接包装器
struct IpcConnection {
    sender: http1::SendRequest<Full<Bytes>>,
    last_used: Instant,
}

impl IpcConnection {
    #[inline]
    fn new(sender: http1::SendRequest<Full<Bytes>>) -> Self {
        Self {
            sender,
            last_used: Instant::now(),
        }
    }

    #[inline]
    fn is_valid(&self) -> bool {
        self.sender.is_ready()
    }
}

// IPC 连接池
#[derive(Clone)]
pub struct IpcConnectionPool {
    connections: Arc<SegQueue<IpcConnection>>,
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
            connections: Arc::new(SegQueue::new()),
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

    // 清理空闲连接
    #[inline]
    fn cleanup_idle_connections(&self) {
        let now = Instant::now();
        let min_to_keep = self.config.min_connections;

        let mut total_checked = 0;
        let mut kept = 0;

        let approx_len = self.connections.len();

        for _ in 0..approx_len {
            if let Some(conn) = self.connections.pop() {
                total_checked += 1;
                if conn.is_valid() {
                    let is_idle_timeout = now.duration_since(conn.last_used) > self.config.idle_timeout;

                    if kept < min_to_keep || !is_idle_timeout {
                        self.connections.push(conn);
                        kept += 1;
                    }
                } else {
                    log::debug!("cleanup: dropping invalid connection");
                }
            } else {
                break;
            }
        }
        log::debug!("Cleanup done: checked {}, kept {}", total_checked, kept);
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
        // 从池中获取连接并检查其有效性
        while let Some(conn) = self.connections.pop() {
            log::debug!("Attempting to reuse connection from pool");
            if conn.is_valid() {
                return Ok((conn, true));
            }
        }

        log::trace!("Pool empty, creating new connection");
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

        tauri::async_runtime::spawn(async move {
            if let Err(err) = conn_driver.await {
                log::error!("IPC Connection Error: {:?}", err);
            }
        });

        Ok(IpcConnection::new(sender))
    }

    pub fn clear_pool(&self) {
        while self.connections.pop().is_some() {}
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

            let hyper_res = match conn.sender.send_request(first_req).await {
                Ok(res) => res,
                Err(first_err) if reused => {
                    // 池中复用的空闲命名管道可能在对端热重载或空闲回收时已半关闭（is_ready 尚未感知），
                    // 自动丢弃旧连接并新建物理管道透明重发 1 次（BUG-329 / BUG-351）。
                    log::debug!("Pooled IPC connection closed ({first_err}), retrying once on fresh pipe");
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

            // 返还连接回连接池
            conn.last_used = Instant::now();
            pool.connections.push(conn);

            Ok(reqwest::Response::from(final_res))
        };

        match timeout_dur {
            Some(d) => timeout(*d, process).await?,
            None => process.await,
        }
    }
}
