//! 服务器模块
//!
//! 提供服务器角色的高级 API。
//!
//! # 职责
//! - ServerManager: 管理vsock监听和连接接受
//! - VirgeServer: 单个连接的数据传输，与VirgeClient类似

use crate::error::{Result, VirgeError};
use crate::transport::Transport;
use log::*;

/// 监听器枚举
enum Listener {
    #[cfg(feature = "use-yamux")]
    Yamux(tokio_vsock::VsockListener),
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

/// Virga 服务器连接：与VirgeClient类似，负责单个连接的数据传输。
pub struct VirgeServer {
    transport: Box<dyn Transport>,
    connected: bool,
}

impl ServerManager {
    pub const fn new(config: ServerConfig) -> Self {
        Self {
            config,
            listener: None,
            running: false,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!(
            "ServerManager starting on cid={}, port={}",
            self.config.listen_cid, self.config.listen_port
        );

        self.listener = Some(self.create_listener().await?);
        self.running = true;
        Ok(())
    }

    async fn create_listener(&self) -> Result<Listener> {
        #[cfg(feature = "use-yamux")]
        {
            let addr = tokio_vsock::VsockAddr::new(self.config.listen_cid, self.config.listen_port);
            let listener = tokio_vsock::VsockListener::bind(addr).map_err(|e| {
                VirgeError::ConnectionError(format!("Failed to bind yamux listener: {}", e))
            })?;
            return Ok(Listener::Yamux(listener));
        }

        #[cfg(feature = "use-xtransport")]
        {
            let addr = vsock::VsockAddr::new(self.config.listen_cid, self.config.listen_port);
            let listener = vsock::VsockListener::bind(&addr).map_err(|e| {
                VirgeError::ConnectionError(format!("Failed to bind xtransport listener: {}", e))
            })?;
            return Ok(Listener::XTransport(listener));
        }

        #[cfg(not(any(feature = "use-yamux", feature = "use-xtransport")))]
        unreachable!("Either use-yamux or use-xtransport feature must be enabled");
    }

    pub async fn accept(&mut self) -> Result<VirgeServer> {
        if !self.running {
            return Err(VirgeError::Other("ServerManager not running".to_string()));
        }

        if let Some(ref mut listener) = self.listener {
            let transport: Box<dyn Transport> = match listener {
                #[cfg(feature = "use-yamux")]
                Listener::Yamux(yamux_listener) => {
                    let (stream, addr) = yamux_listener.accept().await.map_err(|e| {
                        VirgeError::ConnectionError(format!(
                            "Failed to accept yamux connection: {}",
                            e
                        ))
                    })?;
                    info!("Accepted yamux connection from {:?}", addr);

                    // 创建 YamuxTransport 实例并从流初始化
                    let mut transport = Box::new(crate::transport::YamuxTransport::new_server());
                    transport.from_tokio_stream(stream).await?;
                    transport as Box<dyn Transport>
                }

                #[cfg(feature = "use-xtransport")]
                Listener::XTransport(xtransport_listener) => {
                    let (stream, addr) = xtransport_listener.accept().map_err(|e| {
                        VirgeError::ConnectionError(format!(
                            "Failed to accept xtransport connection: {}",
                            e
                        ))
                    })?;
                    info!("Accepted xtransport connection from {:?}", addr);

                    // 创建 XTransportHandler 实例并从流初始化
                    let mut transport = Box::new(crate::transport::XTransportHandler::new());
                    transport
                        .from_stream(stream, self.config.chunk_size, self.config.is_ack)
                        .await?;
                    transport as Box<dyn Transport>
                }
            };

            Ok(VirgeServer {
                transport,
                connected: true,
            })
        } else {
            Err(VirgeError::Other("Listener not initialized".to_string()))
        }
    }

    /// 停止服务器
    pub async fn stop(&mut self) -> Result<()> {
        info!("ServerManager stopping");
        self.listener = None;
        self.running = false;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl VirgeServer {
    /// 发送数据
    pub async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        if !self.connected {
            return Err(VirgeError::TransportError(
                "Server not connected".to_string(),
            ));
        }
        self.transport.send(data).await
    }

    /// 接收数据
    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(VirgeError::TransportError(
                "Server not connected".to_string(),
            ));
        }
        self.transport.recv().await
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        if self.connected {
            self.transport.disconnect().await?;
            self.connected = false;
        }
        Ok(())
    }

    /// 检查连接状态
    pub fn is_connected(&self) -> bool {
        self.connected && self.transport.is_connected()
    }
}
