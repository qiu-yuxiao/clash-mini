#![allow(dead_code)]
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
    time::Duration,
};

use futures_util::StreamExt;
use http::{
    HeaderMap, HeaderValue, Request,
    header::{AUTHORIZATION, CONNECTION, CONTENT_TYPE, HOST, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE},
};
use reqwest::{Method, RequestBuilder};
use serde_json::json;
use tauri::{async_runtime::Mutex, ipc::InvokeResponseBody};
use tokio_tungstenite::{
    client_async, connect_async,
    tungstenite::{Message, client::IntoClientRequest, protocol::CloseFrame as ProtocolCloseFrame},
};

use crate::{
    Error, IpcConnectionPool, Result,
    ipc::LocalSocket,
    models::{
        BaseConfig, ConnectionId, ConnectionManager, Connections, CoreUpdaterChannel, ErrorResponse, Groups, LogLevel,
        MihomoVersion, Protocol, Proxies, Proxy, ProxyDelay, ProxyProvider, ProxyProviders, RuleProviders, Rules,
        WebSocketWriter,
    },
    ret_failed_resp, utils,
};

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

type WsReaderKey = (usize, ConnectionId);

static WS_READER_CANCELLATIONS: LazyLock<Mutex<HashMap<WsReaderKey, tokio::sync::oneshot::Sender<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn ws_reader_key(manager: &Arc<ConnectionManager>, id: ConnectionId) -> WsReaderKey {
    (Arc::as_ptr(manager) as usize, id)
}

fn raw_text_channel_body(text: &str) -> InvokeResponseBody {
    InvokeResponseBody::Raw(text.as_bytes().to_vec())
}

fn websocket_message_to_channel_body(
    message: std::result::Result<Message, tokio_tungstenite::tungstenite::Error>,
) -> (Option<InvokeResponseBody>, bool) {
    match message {
        Ok(Message::Text(text)) => (Some(raw_text_channel_body(&text)), false),
        Ok(Message::Close(_)) => (None, true),
        Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => (None, false),
        Err(err) => {
            log::error!("websocket error: {err}");
            let error_message = Error::from(err).to_string();
            (Some(raw_text_channel_body(&error_message)), true)
        }
    }
}

fn channel_body_to_text_bytes(body: InvokeResponseBody) -> Option<Vec<u8>> {
    match body {
        InvokeResponseBody::Raw(bytes) => Some(bytes),
        InvokeResponseBody::Json(_) => None,
    }
}

fn forward_channel_text<F>(on_message: F) -> impl Fn(InvokeResponseBody) -> bool + Send + 'static
where
    F: Fn(Vec<u8>) + Send + 'static,
{
    move |data| {
        if let Some(bytes) = channel_body_to_text_bytes(data) {
            on_message(bytes);
        }
        true
    }
}

async fn track_ws_reader(key: WsReaderKey, cancel_reader: tokio::sync::oneshot::Sender<()>) {
    WS_READER_CANCELLATIONS.lock().await.insert(key, cancel_reader);
}

async fn cancel_ws_reader(key: WsReaderKey) {
    if let Some(cancel_reader) = WS_READER_CANCELLATIONS.lock().await.remove(&key) {
        let _ = cancel_reader.send(());
    }
}

async fn untrack_ws_reader(key: WsReaderKey) {
    WS_READER_CANCELLATIONS.lock().await.remove(&key);
}

pub struct Mihomo {
    pub protocol: Protocol,
    pub external_host: Option<String>,
    pub external_port: Option<u16>,
    pub secret: Option<String>,
    pub socket_path: Option<String>,
    pub connection_manager: Arc<ConnectionManager>,
}

impl Mihomo {
    pub fn update_protocol(&mut self, protocol: Protocol) {
        self.protocol = protocol;
    }

    #[inline]
    pub fn update_external_host(&mut self, host: Option<String>) {
        self.external_host = host;
    }

    pub fn update_external_port(&mut self, port: Option<u16>) {
        self.external_port = port;
    }

    #[inline]
    pub fn update_secret(&mut self, secret: Option<String>) {
        self.secret = secret;
    }

    #[inline]
    pub fn update_socket_path<S: Into<String>>(&mut self, socket_path: S) -> Result<()> {
        self.socket_path = Some(socket_path.into());
        let pool = IpcConnectionPool::global()?;
        pool.clear_pool();
        Ok(())
    }

    #[inline]
    fn get_req_url(&self, suffix_url: &str) -> Result<String> {
        let suffix_url = suffix_url.trim_start_matches("/");
        match self.protocol {
            Protocol::Http => {
                if let Some(host) = self.external_host.as_ref() {
                    let port = self.external_port.unwrap_or(9090);
                    Ok(format!("http://{host}:{port}/{suffix_url}"))
                } else {
                    log::error!("missing external host parameter");
                    Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing external host".to_string(),
                    )))
                }
            }
            Protocol::LocalSocket => Ok(format!("http://localhost/{suffix_url}")),
        }
    }

    #[inline]
    fn get_req_headers(&self) -> Result<HeaderMap<HeaderValue>> {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str("localhost")?);
        headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json")?);
        if matches!(self.protocol, Protocol::Http)
            && let Some(secret) = &self.secret
        {
            let auth_value = HeaderValue::from_str(&format!("Bearer {secret}"))?;
            headers.insert(AUTHORIZATION, auth_value);
        }
        Ok(headers)
    }

    #[inline]
    fn build_request(&self, method: Method, suffix_url: &str) -> Result<RequestBuilder> {
        let url = self.get_req_url(suffix_url)?;
        let headers = self.get_req_headers()?;
        let client = reqwest::ClientBuilder::new().build()?;
        let req = match method {
            Method::POST => Ok(client.post(url).headers(headers)),
            Method::GET => Ok(client.get(url).headers(headers)),
            Method::PUT => Ok(client.put(url).headers(headers)),
            Method::PATCH => Ok(client.patch(url).headers(headers)),
            Method::DELETE => Ok(client.delete(url).headers(headers)),
            _ => {
                let method_str = method.as_str().to_string();
                log::error!("method not supported: {method_str}");
                Err(Error::MethodNotSupported(method_str))
            }
        };
        // 在此设置 timeout，以供构建 local socket 连接时，获取到 timeout 属性
        Ok(req?.timeout(DEFAULT_REQUEST_TIMEOUT))
    }

    async fn send_by_protocol(&self, client: RequestBuilder) -> Result<reqwest::Response> {
        match self.protocol {
            Protocol::Http => client.send().await.map_err(Error::Reqwest),
            Protocol::LocalSocket => {
                if let Some(socket_path) = self.socket_path.as_ref() {
                    log::debug!("send to local socket: {socket_path}");
                    client.send_by_local_socket(socket_path).await
                } else {
                    log::error!("missing socket path parameter");
                    Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing socket path".to_string(),
                    )))
                }
            }
        }
    }

    #[inline]
    fn get_websocket_url(&self, suffix_url: &str) -> Result<String> {
        let suffix_url = suffix_url.trim_start_matches("/");
        match self.protocol {
            Protocol::Http => {
                if let Some(host) = self.external_host.as_ref() {
                    let port = self.external_port.unwrap_or(9090);
                    let secret = self.secret.as_deref().unwrap_or_default();
                    Ok(format!("ws://{host}:{port}/{suffix_url}?token={secret}"))
                } else {
                    log::error!("missing external host parameter");
                    Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing external host".to_string(),
                    )))
                }
            }
            Protocol::LocalSocket => Ok(format!("ws://localhost/{suffix_url}")),
        }
    }

    /// 连接 WebSocket
    async fn connect<F>(&self, url: String, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(InvokeResponseBody) -> bool + Send + 'static,
    {
        let id = rand::random();
        log::info!("connecting to websocket: {url}, id: {id}");
        let manager = Arc::clone(&self.connection_manager);

        match self.protocol {
            Protocol::Http => {
                log::debug!("starting connect to websocket by using http");
                let request = url.into_client_request()?;
                let (ws_stream, _) = connect_async(request).await?;
                let (writer, mut reader) = ws_stream.split();
                let (cancel_reader, mut cancel_reader_rx) = tokio::sync::oneshot::channel();
                let reader_key = ws_reader_key(&manager, id);

                manager
                    .0
                    .write()
                    .await
                    .insert(id, WebSocketWriter::TcpStreamWriter(writer));
                track_ws_reader(reader_key, cancel_reader).await;

                tokio::spawn(async move {
                    let manager_ = Arc::clone(&manager);
                    loop {
                        log::trace!("waiting for websocket message, connection_id: {id}");
                        tokio::select! {
                            biased;
                            _ = &mut cancel_reader_rx => {
                                log::debug!("connection [{id}] reader cancelled");
                                break;
                            }
                            message = reader.next() => {
                                match message {
                                    Some(message) => {
                                        let (response, should_close) = websocket_message_to_channel_body(message);
                                        if should_close {
                                            log::debug!("connection [{id}] is closed");
                                        }
                                        let keep_reader = response.is_none_or(&on_message);
                                        if should_close || !keep_reader {
                                            if !keep_reader {
                                                log::debug!("message receiver dropped, closing websocket connection [{id}]");
                                            }
                                            manager_.0.write().await.remove(&id);
                                            untrack_ws_reader(reader_key).await;
                                            break;
                                        }
                                    }
                                    None => {
                                        log::debug!("connection [{id}] stream ended");
                                        manager_.0.write().await.remove(&id);
                                        untrack_ws_reader(reader_key).await;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                Ok(id)
            }
            Protocol::LocalSocket => {
                if let Some(socket_path) = self.socket_path.as_ref() {
                    log::debug!("starting connect to websocket by using local socket: {socket_path}");
                    let stream = crate::ipc::connect_to_socket(socket_path).await?;

                    let request = Request::builder()
                        .uri(url)
                        .header(HOST, "clash-verge")
                        .header(SEC_WEBSOCKET_KEY, utils::generate_websocket_key())
                        .header(CONNECTION, "Upgrade")
                        .header(UPGRADE, "websocket")
                        .header(SEC_WEBSOCKET_VERSION, "13")
                        .body(())?;
                    let (ws_stream, _) = client_async(request, stream).await?;
                    let (writer, mut reader) = ws_stream.split();
                    let (cancel_reader, mut cancel_reader_rx) = tokio::sync::oneshot::channel();
                    let reader_key = ws_reader_key(&manager, id);

                    manager
                        .0
                        .write()
                        .await
                        .insert(id, WebSocketWriter::SocketStreamWriter(writer));
                    track_ws_reader(reader_key, cancel_reader).await;

                    tokio::spawn(async move {
                        let manager_ = Arc::clone(&manager);
                        loop {
                            log::trace!("waiting for websocket message, connection_id: {id}");
                            tokio::select! {
                                biased;
                                _ = &mut cancel_reader_rx => {
                                    log::debug!("connection [{id}] reader cancelled");
                                    break;
                                }
                                message = reader.next() => {
                                    match message {
                                    Some(message) => {
                                            let (response, should_close) = websocket_message_to_channel_body(message);
                                            if should_close {
                                                log::debug!("connection [{id}] closed");
                                            }
                                            let keep_reader = response.is_none_or(&on_message);
                                            if should_close || !keep_reader {
                                                if !keep_reader {
                                                    log::debug!("message receiver dropped, closing websocket connection [{id}]");
                                                }
                                                manager_.0.write().await.remove(&id);
                                                untrack_ws_reader(reader_key).await;
                                                break;
                                            }
                                        }
                                        None => {
                                            log::debug!("connection [{id}] stream ended");
                                            manager_.0.write().await.remove(&id);
                                            untrack_ws_reader(reader_key).await;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    });
                    Ok(id)
                } else {
                    log::error!("missing socket path parameter");
                    Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "missing socket path".to_string(),
                    )))
                }
            }
        }
    }

    /// 取消 WebSocket 连接
    pub async fn disconnect(&self, id: ConnectionId, force_timeout: Option<u64>) -> Result<()> {
        log::debug!("disconnecting connection: {id}");
        let Some(mut writer) = self.connection_manager.0.write().await.remove(&id) else {
            log::error!("connection not found: {id}");
            return Err(Error::ConnectionNotFound(id));
        };

        cancel_ws_reader(ws_reader_key(&self.connection_manager, id)).await;
        let close_message = Message::Close(Some(ProtocolCloseFrame {
            code: 1000.into(),
            reason: "Disconnected by client".into(),
        }));

        if let Some(timeout) = force_timeout.filter(|timeout| *timeout > 0) {
            let _ = tokio::time::timeout(Duration::from_millis(timeout), writer.send(close_message)).await;
        } else {
            let _ = writer.send(close_message).await;
        }
        Ok(())
    }

    pub async fn clear_all_ws_connections(&self) -> Result<()> {
        log::debug!("start to clear all websocket connections");
        let mut manager = self.connection_manager.0.write().await;
        log::debug!("manage_ids: {:?}", manager.keys());
        let ids: Vec<_> = manager.keys().copied().collect();
        manager.clear();
        log::debug!("clear all done, manager_ids: {:?}", manager.keys());
        drop(manager);
        for id in ids {
            cancel_ws_reader(ws_reader_key(&self.connection_manager, id)).await;
        }
        Ok(())
    }

    // ------------------------------------------------------
    // |                     Mihomo API                     |
    // ------------------------------------------------------
    /// WebSocket: Mihomo 流量数据
    pub async fn ws_traffic<F>(&self, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        self.ws_traffic_checked(forward_channel_text(on_message)).await
    }

    pub(crate) async fn ws_traffic_checked<F>(&self, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(InvokeResponseBody) -> bool + Send + 'static,
    {
        let ws_url = self.get_websocket_url("/traffic")?;
        self.connect(ws_url, on_message).await
    }

    /// WebSocket: Mihomo 内存使用数据
    pub async fn ws_memory<F>(&self, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        self.ws_memory_checked(forward_channel_text(on_message)).await
    }

    pub(crate) async fn ws_memory_checked<F>(&self, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(InvokeResponseBody) -> bool + Send + 'static,
    {
        let ws_url = self.get_websocket_url("/memory")?;
        self.connect(ws_url, on_message).await
    }

    /// WebSocket: Mihomo 连接信息数据
    pub async fn ws_connections<F>(&self, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        self.ws_connections_checked(forward_channel_text(on_message)).await
    }

    pub(crate) async fn ws_connections_checked<F>(&self, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(InvokeResponseBody) -> bool + Send + 'static,
    {
        let ws_url = self.get_websocket_url("/connections")?;
        self.connect(ws_url, on_message).await
    }

    /// WebSocket: Mihomo 日志数据
    pub async fn ws_logs<F>(&self, level: LogLevel, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(Vec<u8>) + Send + 'static,
    {
        self.ws_logs_checked(level, forward_channel_text(on_message)).await
    }

    pub(crate) async fn ws_logs_checked<F>(&self, level: LogLevel, on_message: F) -> Result<ConnectionId>
    where
        F: Fn(InvokeResponseBody) -> bool + Send + 'static,
    {
        let ws_url = self.get_websocket_url("/logs")?;
        let ws_url = match self.protocol {
            // url 后面添加 format=structured 参数的日志格式如下：
            // {"time":"11:49:58","level":"debug","message":"[DNS] hijack udp:192.168.2.1:53 from 198.18.0.1:42761","fields":[]}
            Protocol::Http => format!("{ws_url}&level={level}"),
            Protocol::LocalSocket => format!("{ws_url}?level={level}"),
        };
        self.connect(ws_url, on_message).await
    }

    // clash api
    /// 获取 Mihomo 版本信息
    pub async fn get_version(&self) -> Result<MihomoVersion> {
        let client = self.build_request(Method::GET, "/version")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("get mihomo version failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<MihomoVersion>().await?)
    }

    /// 清理 FakeIP 缓存
    pub async fn flush_fakeip(&self) -> Result<()> {
        let client = self.build_request(Method::POST, "/cache/fakeip/flush")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("flush fakeip cache failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 清理 DNS 缓存
    pub async fn flush_dns(&self) -> Result<()> {
        let client = self.build_request(Method::POST, "/cache/dns/flush")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("flush dns cache failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 获取全部连接信息
    pub async fn get_connections(&self) -> Result<Connections> {
        let client = self.build_request(Method::GET, "/connections")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("get all connections failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<Connections>().await?)
    }

    /// 关闭全部连接
    pub async fn close_all_connections(&self) -> Result<()> {
        let client = self.build_request(Method::DELETE, "/connections")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("close all connections failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 关闭指定 ID 的连接
    pub async fn close_connection(&self, connection_id: &str) -> Result<()> {
        let client = self.build_request(Method::DELETE, &format!("/connections/{connection_id}"))?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("close connection failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 获取所有的代理组
    pub async fn get_groups(&self) -> Result<Groups> {
        let client = self.build_request(Method::GET, "/group")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("get all groups failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<Groups>().await?)
    }

    /// 获取指定名称的代理组
    pub async fn get_group_by_name(&self, group_name: &str) -> Result<Proxy> {
        let group_name_encode = urlencoding::encode(group_name);
        let client = self.build_request(Method::GET, &format!("/group/{group_name_encode}"))?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("get group[{}] failed, {}", group_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<Proxy>().await?)
    }

    /// 对指定代理组进行延迟测试, 同时清理代理组已固定的节点
    pub async fn delay_group(&self, group_name: &str, test_url: &str, timeout: u32) -> Result<HashMap<String, u32>> {
        let group_name_encode = urlencoding::encode(group_name);
        let test_url = urlencoding::encode(test_url);
        let suffix_url = format!("/group/{group_name_encode}/delay?url={test_url}&timeout={timeout}");
        let req_timeout = Duration::from_millis(timeout as u64) + DEFAULT_REQUEST_TIMEOUT;
        let client = self.build_request(Method::GET, &suffix_url)?.timeout(req_timeout);
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("delay group[{}] failed, {}", group_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<HashMap<String, u32>>().await?)
    }

    /// 获取代理提供者信息
    pub async fn get_proxy_providers(&self) -> Result<ProxyProviders> {
        let client = self.build_request(Method::GET, "/providers/proxies")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("get all proxy providers failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<ProxyProviders>().await?)
    }

    /// 获取指定代理提供者信息
    pub async fn get_proxy_provider_by_name(&self, provider_name: &str) -> Result<ProxyProvider> {
        let provider_name_encode = urlencoding::encode(provider_name);
        let client = self.build_request(Method::GET, &format!("/providers/proxies/{provider_name_encode}"))?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("get proxy provider[{}] failed, {}", provider_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<ProxyProvider>().await?)
    }

    /// 更新指定代理提供者信息
    pub async fn update_proxy_provider(&self, provider_name: &str) -> Result<()> {
        let provider_name_encode = urlencoding::encode(provider_name);
        let client = self.build_request(Method::PUT, &format!("/providers/proxies/{provider_name_encode}"))?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("update proxy provider[{}] failed, {}", provider_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 对指定代理提供者进行健康检查
    pub async fn healthcheck_proxy_provider(&self, provider_name: &str) -> Result<()> {
        let provider_name_encode = urlencoding::encode(provider_name);
        let suffix_url = format!("/providers/proxies/{provider_name_encode}/healthcheck");
        let client = self.build_request(Method::GET, &suffix_url)?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("healthcheck proxy provider[{}] failed, {}", provider_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 对指定代理提供者下的指定节点（非代理组）进行健康检查, 并返回新的延迟信息
    pub async fn healthcheck_node_in_provider(
        &self,
        provider_name: &str,
        proxy_name: &str,
        test_url: &str,
        timeout: u32,
    ) -> Result<ProxyDelay> {
        let provider_name_encode = urlencoding::encode(provider_name);
        let proxy_name_encode = urlencoding::encode(proxy_name);
        let suffix_url = format!("/providers/proxies/{provider_name_encode}/{proxy_name_encode}/healthcheck");
        let req_timeout = Duration::from_millis(timeout as u64) + DEFAULT_REQUEST_TIMEOUT;
        let client = self
            .build_request(Method::GET, &suffix_url)?
            .query(&[("url", test_url), ("timeout", &timeout.to_string())])
            .timeout(req_timeout);
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            // maybe proxy delay is timeout response, try parse it.
            match response.json::<ErrorResponse>().await {
                Ok(err_res) => {
                    log::debug!("healthcheck node[{}] error: {}", proxy_name, err_res.message);
                    return Ok(ProxyDelay { delay: 0 });
                }
                Err(e) => {
                    ret_failed_resp!("healthcheck node[{}] failed, {}", proxy_name, e);
                }
            }
        }
        Ok(response.json::<ProxyDelay>().await?)
    }

    /// 获取所有代理信息
    pub async fn get_proxies(&self) -> Result<Proxies> {
        let client = self.build_request(Method::GET, "/proxies")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("get all proxies failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<Proxies>().await?)
    }

    /// 获取指定代理信息
    pub async fn get_proxy_by_name(&self, proxy_name: &str) -> Result<Proxy> {
        let proxy_name_encode = urlencoding::encode(proxy_name);
        let client = self.build_request(Method::GET, &format!("/proxies/{proxy_name_encode}"))?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("get proxy[{}] failed, {}", proxy_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<Proxy>().await?)
    }

    /// 为指定代理选择节点
    ///
    /// 一般为指定代理组下使用指定的代理节点 【代理组/节点】
    pub async fn select_node_for_group(&self, group_name: &str, node: &str) -> Result<()> {
        let group_name_encode = urlencoding::encode(group_name);
        let body = json!({ "name": node });
        let client = self
            .build_request(Method::PUT, &format!("/proxies/{group_name_encode}"))?
            .json(&body);
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("select node[{}] for group[{}] failed, {}", node, group_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 指定代理组下不再使用固定的代理节点
    ///
    /// 一般用于自动选择的代理组（例如：URLTest 类型的代理组）下的节点
    pub async fn unfixed_proxy(&self, group_name: &str) -> Result<()> {
        let group_name_encode = urlencoding::encode(group_name);
        let client = self.build_request(Method::DELETE, &format!("/proxies/{group_name_encode}"))?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("unfixed group[{}] failed, {}", group_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 对指定代理进行延迟测试
    ///
    /// 一般用于代理节点的延迟测试，也可传代理组名称（只会测试代理组下选中的代理节点）
    pub async fn delay_proxy_by_name(&self, proxy_name: &str, test_url: &str, timeout: u32) -> Result<ProxyDelay> {
        let proxy_name_encode = urlencoding::encode(proxy_name);
        let suffix_url = format!("/proxies/{proxy_name_encode}/delay");
        let req_timeout = Duration::from_millis(timeout as u64) + DEFAULT_REQUEST_TIMEOUT;
        let client = self
            .build_request(Method::GET, &suffix_url)?
            .query(&[("timeout", &timeout.to_string()), ("url", &test_url.to_string())])
            .timeout(req_timeout);
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            match response.json::<ErrorResponse>().await {
                Ok(err_res) => {
                    log::debug!(
                        "delay proxy[{}], mark it timeout, response error message: {}",
                        proxy_name,
                        err_res.message
                    );
                    return Ok(ProxyDelay { delay: 0 });
                }
                Err(e) => {
                    ret_failed_resp!("delay proxy[{}] failed, {}", proxy_name, e);
                }
            }
        }
        Ok(response.json::<ProxyDelay>().await?)
    }

    /// 获取所有规则信息
    pub async fn get_rules(&self) -> Result<Rules> {
        let client = self.build_request(Method::GET, "/rules")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("get all rules failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<Rules>().await?)
    }

    /// 获取所有规则提供者信息
    pub async fn get_rule_providers(&self) -> Result<RuleProviders> {
        let client = self.build_request(Method::GET, "/providers/rules")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("get all rule providers failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<RuleProviders>().await?)
    }

    /// 更新规则提供者信息
    pub async fn update_rule_provider(&self, provider_name: &str) -> Result<()> {
        let provider_name_encode = urlencoding::encode(provider_name);
        let client = self.build_request(Method::PUT, &format!("/providers/rules/{provider_name_encode}"))?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("update rule provider[{}] failed, {}", provider_name, e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 获取基础配置
    pub async fn get_base_config(&self) -> Result<BaseConfig> {
        let client = self.build_request(Method::GET, "/configs")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("get base config failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(response.json::<BaseConfig>().await?)
    }

    /// 重新加载配置
    pub async fn reload_config(&self, force: bool, config_path: &str) -> Result<()> {
        let body = json!({ "path": config_path });
        let client = self
            .build_request(Method::PUT, "/configs")?
            .timeout(Duration::from_secs(60))
            .query(&[("force", force)])
            .json(&body);
        let response_result = self.send_by_protocol(client).await;
        if matches!(self.protocol, Protocol::LocalSocket)
            && let Ok(pool) = IpcConnectionPool::global()
        {
            pool.clear_pool();
        }
        let response = response_result?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("reload base config failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 更新基础配置
    pub async fn patch_base_config<D: serde::Serialize + Clone + Sync>(&self, data: &D) -> Result<()> {
        let client = { self.build_request(Method::PATCH, "/configs")?.json(&data) };
        let response = { self.send_by_protocol(client).await? };
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("patch base config failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 更新 Geo, 同 [`upgrade_geo`](crate::mihomo::Mihomo::upgrade_geo)
    pub async fn update_geo(&self) -> Result<()> {
        let client = self
            .build_request(Method::POST, "/configs/geo")?
            .timeout(Duration::from_secs(60));
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("update geo database failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 重启核心
    pub async fn restart(&self) -> Result<()> {
        let client = self.build_request(Method::POST, "/restart")?;
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("restart core failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 升级核心
    pub async fn upgrade_core(&self, channel: CoreUpdaterChannel, force: bool) -> Result<()> {
        let client = self
            .build_request(Method::POST, "/upgrade")?
            .timeout(Duration::from_secs(60))
            .query(&[("channel", &channel.to_string()), ("force", &force.to_string())]);
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("upgrade core failed, {}", e),
                |err_res| {
                    let msg = err_res.message;
                    if msg.to_lowercase().contains("already using latest version") {
                        "already using latest version".to_string()
                    } else {
                        msg
                    }
                },
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 更新 UI
    pub async fn upgrade_ui(&self) -> Result<()> {
        let client = self
            .build_request(Method::POST, "/upgrade/ui")?
            .timeout(Duration::from_secs(60));
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response
                .json::<ErrorResponse>()
                .await
                .map_or_else(|e| format!("upgrade ui failed, {}", e), |err_res| err_res.message);
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }

    /// 更新 Geo
    pub async fn upgrade_geo(&self) -> Result<()> {
        let client = self
            .build_request(Method::POST, "/upgrade/geo")?
            .timeout(Duration::from_secs(60));
        let response = self.send_by_protocol(client).await?;
        if !response.status().is_success() {
            let err_msg = response.json::<ErrorResponse>().await.map_or_else(
                |e| format!("upgrade geo database failed, {}", e),
                |err_res| err_res.message,
            );
            ret_failed_resp!("{}", err_msg);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[derive(serde::Serialize)]
    #[serde(tag = "type", content = "data")]
    enum OldChannelMessage {
        Text(String),
    }

    fn old_channel_json(payload: &str) -> serde_json::Result<String> {
        let value = serde_json::to_value(OldChannelMessage::Text(payload.to_string()))?;
        serde_json::to_string(&value)
    }

    fn raw_channel_body_len(payload: &str) -> usize {
        match raw_text_channel_body(payload) {
            InvokeResponseBody::Raw(bytes) => {
                let len = bytes.len();
                std::hint::black_box(bytes);
                len
            }
            InvokeResponseBody::Json(_) => unreachable!("text websocket messages are sent as raw bytes"),
        }
    }

    fn sample_connections_payload(min_len: usize) -> String {
        let connection = r#"{"id":"bench-id","metadata":{"network":"tcp","type":"HTTP","sourceIP":"198.18.0.1","destinationIP":"93.184.216.34","host":"example.com","dnsMode":"normal","processPath":"/Applications/Example.app"},"chains":["Proxy","DIRECT"],"rule":"MATCH","rulePayload":"","upload":123456,"download":654321,"start":"2026-05-25T00:00:00Z"}"#;
        let mut payload = String::from(r#"{"downloadTotal":1,"uploadTotal":2,"connections":["#);

        while payload.len() < min_len {
            if !payload.ends_with('[') {
                payload.push(',');
            }
            payload.push_str(connection);
        }

        payload.push_str("]}");
        payload
    }

    #[test]
    fn raw_channel_body_can_be_counted_without_json_reparse() -> std::result::Result<(), String> {
        let payload = r#"{"connections":[{"id":"a","metadata":{"host":"example.com"}}]}"#;
        let bytes = channel_body_to_text_bytes(raw_text_channel_body(payload))
            .ok_or_else(|| "raw text channel body did not produce bytes".to_string())?;

        assert_eq!(bytes, payload.as_bytes());
        Ok(())
    }

    #[test]
    #[ignore]
    fn compare_websocket_message_serialization() -> serde_json::Result<()> {
        let iterations = std::env::var("WS_SERIALIZATION_ITERS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(5_000);
        let payload = sample_connections_payload(64 * 1024);

        let old_started = Instant::now();
        let mut old_len = 0usize;
        for _ in 0..iterations {
            let value = serde_json::to_value(OldChannelMessage::Text(std::hint::black_box(payload.clone())))?;
            let json = serde_json::to_string(&value)?;
            old_len = old_len.wrapping_add(std::hint::black_box(json.len()));
        }
        let old_elapsed = old_started.elapsed();

        let raw_started = Instant::now();
        let mut raw_len = 0usize;
        for _ in 0..iterations {
            raw_len = raw_len.wrapping_add(std::hint::black_box(raw_channel_body_len(std::hint::black_box(
                &payload,
            ))));
        }
        let raw_elapsed = raw_started.elapsed();

        println!(
            "payload={}B iterations={} old={:?} raw={:?} raw_speedup={:.2}x old_len={} raw_len={}",
            payload.len(),
            iterations,
            old_elapsed,
            raw_elapsed,
            old_elapsed.as_secs_f64() / raw_elapsed.as_secs_f64(),
            old_len,
            raw_len
        );
        Ok(())
    }
}
