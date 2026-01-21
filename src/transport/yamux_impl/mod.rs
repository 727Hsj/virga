//! Yamux 传输协议实现
//!
//! 基于 yamux 库的多路复用传输实现。
//!
//! # 特点
//! - 支持多个独立的虚拟流
//! - 适合多并发场景
//! - 由 libp2p 社区维护
//!
//! # 结构
//! ```text
//! ┌─────────────────────────────────┐
//! │ YamuxTransport                  │
//! │ - connection: Option<Connection>│
//! │ - yamux_stream: Option<Stream>  │
//! └─────────────────────────────────┘
//! ```

use crate::error::{Result, VirgeError};
use crate::transport::Transport;
use async_trait::async_trait;
use futures::AsyncReadExt;
use futures::AsyncWriteExt;
use futures::future::poll_fn;
use log::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use tokio_vsock::{VsockAddr, VsockStream};

use yamux::Stream;
use yamux::{Config, Connection, Mode};

/// Yamux 传输协议实现
///
/// 直接管理 tokio-vsock 连接并使用 yamux 进行多路复用。
/// Yamux需要持续的驱动程序来处理入站流和连接生命周期。
pub struct YamuxTransport {
    yamux_stream: Option<Stream>,
    connection: Option<Arc<Mutex<Connection<Compat<VsockStream>>>>>,
    driver_handle: Option<tokio::task::JoinHandle<()>>,
    is_server: bool,
}

impl YamuxTransport {
    /// 创建客户端模式的 Yamux 传输实例
    pub fn new_client() -> Self {
        Self {
            connection: None,
            yamux_stream: None,
            driver_handle: None,
            is_server: false,
        }
    }

    /// 创建服务器模式的 Yamux 传输实例
    pub fn new_server() -> Self {
        Self {
            connection: None,
            yamux_stream: None,
            driver_handle: None,
            is_server: true,
        }
    }

    /// 获取或创建 yamux 虚拟流
    async fn get_or_create_stream(&mut self) -> Result<&mut Stream> {
        if self.yamux_stream.is_none() {
            if self.is_server {
                // 服务器模式：等待从驱动程序接收入站流
                if let Some(connection_arc) = self.connection.clone() {
                    let mut conn_guard = connection_arc.lock().await;
                    let stream = poll_fn(|cx| conn_guard.poll_next_inbound(cx)).await;
                    match stream {
                        Some(Ok(yamux_stream)) => {
                            self.yamux_stream = Some(yamux_stream);
                        }
                        Some(Err(e)) => {
                            return Err(VirgeError::TransportError(format!(
                                "Failed to open yamux stream: {}",
                                e
                            )));
                        }
                        None => {
                            return Err(VirgeError::TransportError(
                                "Failed to open yamux stream".to_string(),
                            ));
                        }
                    }
                } else {
                    return Err(VirgeError::TransportError(
                        "Yamux not initialized".to_string(),
                    ));
                }
            } else {
                // 客户端模式：创建出站流
                if let Some(connection_arc) = self.connection.clone() {
                    let mut conn_guard = connection_arc.lock().await;
                    let stream = poll_fn(|cx| conn_guard.poll_new_outbound(cx))
                        .await
                        .map_err(|e| {
                            VirgeError::TransportError(format!(
                                "Failed to open yamux stream: {}",
                                e
                            ))
                        })?;
                    info!("Client created outbound stream: {:?}", stream.id());
                    self.yamux_stream = Some(stream);
                } else {
                    return Err(VirgeError::TransportError(
                        "Yamux not initialized".to_string(),
                    ));
                }
            }
        }

        Ok(self.yamux_stream.as_mut().unwrap())
    }

    /// yamux 连接驱动程序
    fn start_driver(&mut self) {
        if let Some(conn_arc) = self.connection.clone() {
            let driver_handle = tokio::spawn(async move {
                debug!("Starting yamux connection driver");
                loop {
                    let mut conn_guard = conn_arc.lock().await;
                    match poll_fn(|cx| conn_guard.poll_next_inbound(cx)).await {
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            debug!("Yamux connection error: {}", e);
                            break;
                        }
                        None => {
                            debug!("Yamux connection closed");
                            break;
                        }
                    }
                    drop(conn_guard);
                }
                info!("Yamux connection driver stopped");
            });

            self.driver_handle = Some(driver_handle);
        }
    }
}

#[async_trait]
impl Transport for YamuxTransport {
    async fn connect(&mut self, cid: u32, port: u32, _: u32, _: bool) -> Result<()> {
        info!("Yamux transport connecting to cid={}, port={}", cid, port);

        let stream = VsockStream::connect(VsockAddr::new(cid, port))
            .await
            .map_err(|e| VirgeError::ConnectionError(format!("Failed to connect vsock: {}", e)))?;

        // 初始化 yamux
        let config = Config::default();
        let connection = Connection::new(stream.compat(), config, Mode::Client);
        self.connection = Some(Arc::new(Mutex::new(connection)));

        // 启动驱动程序来处理连接生命周期
        self.start_driver();
        // 创建yamux_stream
        let _ = self.get_or_create_stream().await?;

        info!("Yamux transport connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Yamux transport disconnecting");

        // 清理驱动程序
        if let Some(handle) = self.driver_handle.take() {
            handle.abort();
        }

        // 清理资源
        self.connection = None;
        self.yamux_stream = None;

        info!("Yamux transport disconnected");
        Ok(())
    }

    async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        if !self.is_connected() {
            return Err(VirgeError::TransportError(
                "Yamux transport not connected about send".to_string(),
            ));
        }

        let stream = self.get_or_create_stream().await?;
        stream
            .write_all(&data)
            .await
            .map_err(|e| VirgeError::Other(format!("yamux send error: {}", e)))?;
        stream.close().await?;

        info!("Yamux sent {} bytes", data.len());
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(VirgeError::TransportError(
                "Yamux transport not connected about recv".to_string(),
            ));
        }
        let stream = self.get_or_create_stream().await?;
        let mut buf = Vec::new();
        stream
            .read_to_end(&mut buf)
            .await
            .map_err(|e| VirgeError::Other(format!("yamux recv error: {}", e)))?;
        info!("Yamux received {} bytes", buf.len());
        Ok(buf)
    }

    fn is_connected(&self) -> bool {
        self.yamux_stream.is_some() && self.connection.is_some()
    }

    async fn from_tokio_stream(&mut self, stream: tokio_vsock::VsockStream) -> Result<()> {
        // 初始化 yamux
        let config = Config::default();
        let connection = Connection::new(stream.compat(), config, Mode::Server);

        self.connection = Some(Arc::new(Mutex::new(connection)));

        // 启动驱动程序来处理连接生命周期
        self.start_driver();
        // 创建yamux_stream
        let _ = self.get_or_create_stream().await?;

        info!("Yamux transport initialized from stream successfully");
        Ok(())
    }
}
