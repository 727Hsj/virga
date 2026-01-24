//! 服务器模块
//!
//! 提供服务器角色的高级 API。
//!
//! # 职责
//! - ServerManager: 管理vsock监听和连接接受
//! - VirgeServer: 单个连接的数据传输，与VirgeClient类似

#[cfg(feature = "use-xtransport")]
pub mod server_sync;
#[cfg(feature = "use-xtransport")]
pub use crate::transport::XTransportHandler;
#[cfg(feature = "use-xtransport")]
pub use server_sync::VirgeServer;

use log::*;
use std::io::{Error, ErrorKind, Result};

/// 监听器枚举
enum Listener {
    #[cfg(feature = "use-xtransport")]
    XTransport(vsock::VsockListener),
}

/// 服务器配置
#[derive(Clone, Debug)]
pub struct ServerConfig {
    listen_cid: u32,
    listen_port: u32,
    chunk_size: u32,
    is_ack: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_cid: crate::VMADDR_CID_ANY as u32,
            listen_port: crate::DEFAULT_SERVER_PORT as u32,
            chunk_size: crate::DEAFULT_CHUNK_SIZE as u32,
            is_ack: crate::DEFAULT_IS_ACK,
        }
    }
}

impl ServerConfig {
    pub fn new(cid: u32, port: u32, chunk: u32, isack: bool) -> Self {
        Self {
            listen_cid: cid,
            listen_port: port,
            chunk_size: chunk,
            is_ack: isack,
        }
    }
}

/// 服务器管理器：负责管理vsock监听和连接接受，为每个连接生成VirgeServer实例
pub struct ServerManager {
    config: ServerConfig,
    listener: Option<Listener>,
    running: bool,
}

impl ServerManager {
    pub const fn new(config: ServerConfig) -> Self {
        Self {
            config,
            listener: None,
            running: false,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        info!(
            "ServerManager starting on cid={}, port={}",
            self.config.listen_cid, self.config.listen_port
        );

        self.listener = Some(self.create_listener()?);
        self.running = true;
        Ok(())
    }

    fn create_listener(&self) -> Result<Listener> {
        #[cfg(feature = "use-xtransport")]
        {
            let addr = vsock::VsockAddr::new(self.config.listen_cid, self.config.listen_port);
            let listener = vsock::VsockListener::bind(&addr)?;
            return Ok(Listener::XTransport(listener));
        }
    }

    pub fn accept(&mut self) -> Result<VirgeServer> {
        if !self.running {
            return Err(Error::new(
                ErrorKind::Other,
                format!("ServerManager not running"),
            ));
        }

        if let Some(ref mut listener) = self.listener {
            let transport = match listener {
                #[cfg(feature = "use-xtransport")]
                Listener::XTransport(xtransport_listener) => {
                    let (stream, addr) = xtransport_listener.accept()?;
                    info!("Accepted xtransport connection from {:?}", addr);

                    // 创建 XTransportHandler 实例并从流初始化
                    let mut transport = XTransportHandler::new();
                    transport.from_stream(stream, self.config.chunk_size, self.config.is_ack)?;
                    transport
                }
            };

            Ok(VirgeServer::new(transport, true))
        } else {
            Err(Error::other(format!("Listener not initialized")))
        }
    }

    /// 停止服务器
    pub fn stop(&mut self) -> Result<()> {
        info!("ServerManager stopping");
        self.listener = None;
        self.running = false;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}
