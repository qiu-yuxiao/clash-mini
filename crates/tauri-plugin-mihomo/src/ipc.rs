use std::{
    ops::{Deref, DerefMut},
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
#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Semaphore, SemaphorePermit},
    time::timeout,
};
#[cfg(windows)]
use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

use crate::{Error, Result};

#[pin_project(project = WrapStreamProj)]
pub enum WrapStream {
    #[cfg(unix)]
    Unix(#[pin] UnixStream),
    #[cfg(windows)]
    NamedPipe(#[pin] NamedPipeClient),
}

impl WrapStream {
    #[inline]
    pub fn is_available(&self) -> Result<bool> {
        match self {
            #[cfg(unix)]
            WrapStream::Unix(s) => {
                let mut buf = [0u8; 1];
                match s.try_io(tokio::io::Interest::READABLE, || {
                    let raw_fd = std::os::unix::io::AsRawFd::as_raw_fd(s);
                    let n = unsafe { libc::recv(raw_fd, buf.as_mut_ptr() as *mut libc::c_void, 1, libc::MSG_PEEK) };
                    if n == 0 {
                        return Err(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "Closed"));
                    }
                    if n < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(n)
                }) {
                    Ok(_) => Ok(true),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            #[cfg(windows)]
            WrapStream::NamedPipe(s) => {
                let mut buffer = [];
                match s.try_read(&mut buffer) {
                    Ok(_) => Ok(true),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(true),
                    Err(_) => Ok(false),
                }
            }
        }
    }

    pub async fn readable(&self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            WrapStream::Unix(s) => s.readable().await,
            #[cfg(windows)]
            WrapStream::NamedPipe(s) => s.readable().await,
        }
    }
    pub async fn writable(&self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            WrapStream::Unix(s) => s.writable().await,
            #[cfg(windows)]
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
            #[cfg(unix)]
            WrapStreamProj::Unix(s) => s.poll_read(cx, buf),
            #[cfg(windows)]
            WrapStreamProj::NamedPipe(s) => s.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for WrapStream {
    #[inline]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.project() {
            #[cfg(unix)]
            WrapStreamProj::Unix(s) => s.poll_write(cx, buf),
            #[cfg(windows)]
            WrapStreamProj::NamedPipe(s) => s.poll_write(cx, buf),
        }
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            #[cfg(unix)]
            WrapStreamProj::Unix(s) => s.poll_flush(cx),
            #[cfg(windows)]
            WrapStreamProj::NamedPipe(s) => s.poll_flush(cx),
        }
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            #[cfg(unix)]
            WrapStreamProj::Unix(s) => s.poll_shutdown(cx),
            #[cfg(windows)]
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

pub async fn connect_to_socket(socket_path: &str) -> Result<WrapStream> {
    #[cfg(unix)]
    {
        use std::io::ErrorKind;
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY: Duration = Duration::from_millis(25);

        let mut last_err: Option<std::io::Error> = None;

        for attempt in 0..=MAX_RETRIES {
            let connect_fut = tokio::time::timeout(Duration::from_millis(200), UnixStream::connect(socket_path));
            let connection_result = match connect_fut.await {
                Ok(result) => result,
                Err(_) => {
                    log::warn!("Socket connect attempt {attempt} timed out");
                    last_err = Some(std::io::Error::new(ErrorKind::TimedOut, "connect timeout"));
                    continue;
                }
            };

            match connection_result {
                Ok(stream) => return Ok(WrapStream::Unix(stream)),
                Err(e) => match e.kind() {
                    ErrorKind::PermissionDenied => {
                        log::error!("Permission denied for socket: {socket_path}");
                        return Err(Error::Io(e));
                    }
                    _ => {
                        log::warn!("Socket connect attempt {attempt} failed: {e}");
                        last_err = Some(e);
                    }
                },
            }

            if attempt < MAX_RETRIES {
                let delay = BASE_DELAY * 2u32.pow(attempt);
                let jitter = Duration::from_millis(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| (d.as_nanos() % 25) as u64)
                        .unwrap_or(0),
                );

                tokio::time::sleep(delay + jitter).await;
            }
        }

        Err(Error::Io(
            last_err.unwrap_or_else(|| std::io::Error::other("Retries exhausted")),
        ))
    }

    #[cfg(windows)]
    {
        let mut max_retry_count = 3;
        const RETRY_DELAY: Duration = Duration::from_millis(125);

        let client = loop {
            match ClientOptions::new().open(socket_path) {
                Ok(client) => break client,
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => (),
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
            tokio::time::sleep(RETRY_DELAY).await;
        };
        Ok(WrapStream::NamedPipe(client))
    }
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

// IPC 连接包装器
struct IpcConnection {
    stream: WrapStream,
    last_used: Instant,
}

impl Deref for IpcConnection {
    type Target = WrapStream;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl DerefMut for IpcConnection {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl IpcConnection {
    #[inline]
    fn new(stream: WrapStream) -> Self {
        Self {
            stream,
            last_used: Instant::now(),
        }
    }

    #[inline]
    fn is_valid(&self) -> bool {
        self.stream.is_available().unwrap_or_default()
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
                let is_idle_timeout = now.duration_since(conn.last_used) > self.config.idle_timeout;

                if kept < min_to_keep || !is_idle_timeout {
                    self.connections.push(conn);
                    kept += 1;
                }
            } else {
                break;
            }
        }
        log::debug!("Cleanup done: checked {}, kept {}", total_checked, kept);
    }

    #[inline]
    async fn get_connection<'a>(&'a self, socket_path: &str) -> Result<(IpcConnection, SemaphorePermit<'a>)> {
        log::debug!("get connection from pool");
        // 确保获取 semaphore permit
        let permit = self.acquire_permit().await?;
        // 开始创建连接
        let conn = self.acquire_or_create_connection(socket_path).await?;
        Ok((conn, permit))
    }

    async fn acquire_permit<'a>(&'a self) -> Result<SemaphorePermit<'a>> {
        log::debug!("acquire permit");
        match self.semaphore.try_acquire() {
            Ok(permit) => Ok(permit),
            Err(_) => match self.config.reject_policy {
                RejectPolicy::New => {
                    log::debug!("max permit has acquire, add permit");
                    self.semaphore.add_permits(1);
                    match self.semaphore.acquire().await {
                        Ok(permit) => Ok(permit),
                        Err(e) => {
                            log::error!("failed to acquire permit, forget permit");
                            self.semaphore.forget_permits(1);
                            Err(Error::ConnectionFailed(e.to_string()))
                        }
                    }
                }
                RejectPolicy::Reject => Err(Error::ConnectionPoolFull),
                RejectPolicy::Timeout(timeout_duration) => {
                    let acquire_future = self.semaphore.acquire();
                    match timeout(timeout_duration, acquire_future).await {
                        Ok(Ok(permit)) => Ok(permit),
                        Ok(Err(_)) => Err(Error::ConnectionPoolFull),
                        Err(e) => Err(Error::Timeout(e)),
                    }
                }
                RejectPolicy::Wait => {
                    let acquire_future = self.semaphore.acquire().await;
                    match acquire_future {
                        Ok(permit) => Ok(permit),
                        Err(_) => Err(Error::ConnectionPoolFull),
                    }
                }
            },
        }
    }

    async fn acquire_or_create_connection(&self, socket_path: &str) -> Result<IpcConnection> {
        // 从池中获取连接并检查其有效性
        while let Some(conn) = self.connections.pop() {
            log::debug!("Attempting to reuse connection from pool");
            if conn.is_valid() {
                return Ok(conn);
            }
            // log::debug!("Pooled connection is invalid, dropping...");
        }

        log::trace!("Pool empty, creating new connection");
        Self::create_connection(socket_path).await
    }

    async fn create_connection(socket_path: &str) -> Result<IpcConnection> {
        log::trace!(
            "creating connection, available permits: {}",
            Self::global()?.semaphore.available_permits()
        );
        match connect_to_socket(socket_path).await {
            Ok(stream) => Ok(IpcConnection::new(stream)),
            Err(e) => Err(Error::ConnectionFailed(e.to_string())),
        }
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

        let pool = IpcConnectionPool::global()?;
        let (conn, _permit) = pool.get_connection(socket_path).await?;

        let method = reqwest_req.method();
        let url = reqwest_req.url();
        let headers = reqwest_req.headers().clone();

        let body_bytes = if let Some(body) = reqwest_req.body() {
            body.as_bytes().map(Bytes::copy_from_slice).unwrap_or_else(Bytes::new)
        } else {
            Bytes::new()
        };

        let mut builder = http::Request::builder().method(method).uri(url.as_str());

        if let Some(h) = builder.headers_mut() {
            *h = headers;
        }
        let hyper_req = builder.body(Full::new(body_bytes))?;

        let process = async move {
            let (mut sender, conn_driver) = http1::handshake(conn.stream)
                .await
                .map_err(|e| Error::HttpParseError(e.to_string()))?;

            tauri::async_runtime::spawn(async move {
                if let Err(err) = conn_driver.await {
                    log::error!("IPC Connection Error: {:?}", err);
                }
            });

            let hyper_res = sender
                .send_request(hyper_req)
                .await
                .map_err(|e| Error::HttpParseError(e.to_string()))?;

            let (res_parts, res_body) = hyper_res.into_parts();
            let collected_body = res_body
                .collect()
                .await
                .map_err(|e| Error::HttpParseError(e.to_string()))?
                .to_bytes();

            let final_res = http::Response::from_parts(res_parts, collected_body);

            Ok(reqwest::Response::from(final_res))
        };

        match timeout_dur {
            Some(d) => timeout(*d, process).await?,
            None => process.await,
        }
    }
}
