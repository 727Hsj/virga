//! 客户端模块
//!
//! 提供客户端角色的高级 API。
//!
//! # 职责
//! - 封装客户端的连接逻辑
//! - 提供简洁的发送/接收接口
//! - 管理传输协议选择

use crate::error::Result;
use crate::transport::Transport;
use log::*;

/// 客户端配置
#[derive(Clone, Debug)]
pub struct ClientConfig {
    server_cid: u32,
    server_port: u32,
    chunk_size: u32,
    is_ack: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_cid: crate::DEFAULT_SERVER_CID as u32,
            server_port: crate::DEFAULT_SERVER_PORT as u32,
            chunk_size: crate::DEAFULT_CHUNK_SIZE as u32,
            is_ack: crate::DEFAULT_IS_ACK,
        }
    }
}

impl ClientConfig {
    pub fn new(cid: u32, port: u32, chunk: u32, isack: bool) -> Self {
        Self {
            server_cid: cid,
            server_port: port,
            chunk_size: chunk,
            is_ack: isack,
        }
    }
}

/// Virga 客户端：提供基于选定传输协议的高级客户端接口。
pub struct VirgeClient {
    transport: Box<dyn Transport>,
    config: ClientConfig,
    connected: bool,
}

impl VirgeClient {
    pub fn new(config: ClientConfig) -> Self {
        #[cfg(feature = "use-xtransport")]
        if cfg!(feature = "use-xtransport") {
            return Self::with_xtransport(config);
        }
        #[cfg(feature = "use-yamux")]
        if cfg!(feature = "use-yamux") {
            return Self::with_yamux(config);
        }
        panic!("Either use-yamux or use-xtransport feature must be enabled");
    }

    #[cfg(feature = "use-yamux")]
    pub fn with_yamux(config: ClientConfig) -> Self {
        Self {
            transport: Box::new(crate::transport::YamuxTransport::new_client()),
            config,
            connected: false,
        }
    }

    #[cfg(feature = "use-xtransport")]
    pub fn with_xtransport(config: ClientConfig) -> Self {
        Self {
            transport: Box::new(crate::transport::XTransportHandler::new()),
            config,
            connected: false,
        }
    }

    /// 建立连接
    pub async fn connect(&mut self) -> Result<()> {
        info!(
            "VirgeClient connecting to cid={}, port={}",
            self.config.server_cid, self.config.server_port
        );

        self.transport
            .connect(
                self.config.server_cid,
                self.config.server_port,
                self.config.chunk_size,
                self.config.is_ack,
            )
            .await?;
        self.connected = true;
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        info!("VirgeClient disconnecting");
        self.transport.disconnect().await?;
        self.connected = false;
        Ok(())
    }

    /// 发送数据
    pub async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        if !self.connected {
            return Err(crate::error::VirgeError::Other(
                "Client not connected".to_string(),
            ));
        }

        self.transport.send(data).await?;
        Ok(())
    }

    /// 接收数据
    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(crate::error::VirgeError::Other(
                "Client not connected".to_string(),
            ));
        }

        self.transport.recv().await
    }

    /// 检查连接状态
    pub fn is_connected(&self) -> bool {
        self.connected && self.transport.is_connected()
    }
}
