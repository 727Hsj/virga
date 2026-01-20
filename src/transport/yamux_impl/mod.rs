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
use futures::future::poll_fn;
use futures::AsyncReadExt;
use futures::AsyncWriteExt;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio_vsock::VsockStream;
use log::*;

use yamux::{Config, Connection, Mode};
use yamux::Stream;


/// Yamux 传输协议实现
///
/// 直接管理 tokio-vsock 连接并使用 yamux 进行多路复用。
pub struct YamuxTransport {
    /// 当前使用的 yamux 虚拟流
    yamux_stream: Option<Stream>,

    /// yamux 连接
    connection: Option<Connection<tokio_util::compat::Compat<VsockStream>>>,
}

impl YamuxTransport {
    /// 创建客户端模式的 Yamux 传输实例
    pub fn new_client() -> Self {
        Self {
            connection: None,
            yamux_stream: None,
        }
    }

    /// 创建服务器模式的 Yamux 传输实例
    pub fn new_server() -> Self {
        Self {
            connection: None,
            yamux_stream: None,
        }
    }

    /// 获取或创建 yamux 虚拟流
    async fn get_or_create_stream(&mut self) -> Result<&mut Stream> {
        if self.yamux_stream.is_none() {
            if let Some(connection) = &mut self.connection {
                // 打开新的虚拟流
                let stream = poll_fn(|cx| connection.poll_new_outbound(cx)).await
                    .map_err(|e| VirgeError::TransportError(format!("Failed to open yamux stream: {}", e)))?;
                self.yamux_stream = Some(stream);
            } else {
                return Err(VirgeError::TransportError("Yamux not initialized".to_string()));
            }
        }

        Ok(self.yamux_stream.as_mut().unwrap())
    }
}

#[async_trait]
impl Transport for YamuxTransport {
    async fn connect(&mut self, cid: u32, port: u32) -> Result<()> {
        info!("Yamux transport connecting to cid={}, port={}", cid, port);

        // 建立 vsock 连接
        let stream = VsockStream::connect(tokio_vsock::VsockAddr::new(cid, port))
            .await
            .map_err(|e| VirgeError::ConnectionError(format!("Failed to connect vsock: {}", e)))?;

        // 初始化 yamux
        let config = Config::default();
        let connection = Connection::new(stream.compat(), config, Mode::Client);

        self.connection = Some(connection);

        info!("Yamux transport connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Yamux transport disconnecting");

        // 清理资源
        self.connection = None;
        self.yamux_stream = None;

        info!("Yamux transport disconnected");
        Ok(())
    }

    async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        if !self.is_connected() {
            return Err(VirgeError::TransportError(
                "Yamux transport not connected".to_string(),
            ));
        }

        let stream = self.get_or_create_stream().await?;
        stream.write_all(&data).await
            .map_err(|e| VirgeError::Other(format!("yamux send error: {}", e)))?;
        stream.close().await?;

        debug!("Yamux sent {} bytes", data.len());
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(VirgeError::TransportError(
                "Yamux transport not connected".to_string(),
            ));
        }

        let stream = self.get_or_create_stream().await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await
            .map_err(|e| VirgeError::Other(format!("yamux recv error: {}", e)))?;

        debug!("Yamux received {} bytes", buf.len());
        Ok(buf)
    }

    fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    async fn from_tokio_stream(&mut self, stream: tokio_vsock::VsockStream) -> Result<()> {
        info!("Yamux transport initializing from existing tokio stream");

        // 初始化 yamux
        let config = yamux::Config::default();
        let connection = Connection::new(stream.compat(), config, yamux::Mode::Server);

        self.connection = Some(connection);

        info!("Yamux transport initialized from stream successfully");
        Ok(())
    }
}
