//! 服务器模块
//!
//! 提供服务器角色的高级 API。
//!
//! # 职责
//! - 封装服务器的监听逻辑
//! - 处理客户端连接
//! - 为每个连接创建独立的 Transport 实例
//! - 管理传输协议选择
//!
//! # 设计思路
//! ```text
//! ┌────────────────────────────────────┐
//! │ VirgeServer                        │
//! │ - config: ServerConfig             │
//! │ - listener: Option<VsockListener>  │
//! │ - transport_factory: TransportType │
//! └────────────────────────────────────┘
//!          │
//!          ├─ listen()
//!          ├─ accept() -> Box<dyn Transport>
//!          └─ stop()
//! ```
//!
//! # 使用示例
//! ```ignore
//! let mut server = VirgeServer::with_yamux(config);
//! server.listen().await?;
//!
//! while let Ok(mut transport) = server.accept().await {
//!     tokio::spawn(async move {
//!         let data = transport.recv().await?;
//!         transport.send(data).await?;
//!         Ok::<(), Box<dyn std::error::Error>>(())
//!     });
//! }
//! ```

use log::*;
use crate::error::Result;
use crate::transport::Transport;


/// 服务器配置
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// 服务器监听的 CID
    pub listen_cid: u32,

    /// 服务器监听的端口
    pub listen_port: u32,

    /// 最大并发连接数
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_cid: crate::VMADDR_CID_ANY as u32,
            listen_port: crate::DEFAULT_SERVER_PORT as u32,
            max_connections: 100,
        }
    }
}


/// 监听器枚举
enum Listener {
    #[cfg(feature = "use-yamux")]
    Yamux(tokio_vsock::VsockListener),
    #[cfg(feature = "use-xtransport")]
    XTransport(vsock::VsockListener),
}

/// Virga 服务器
///
/// 提供基于选定传输协议的高级服务器接口。
pub struct VirgeServer {
    /// 服务器配置
    config: ServerConfig,

    /// vsock 监听器
    listener: Option<Listener>,

    /// 监听状态
    listening: bool,
}

impl VirgeServer {
    /// 创建新的服务器实例
    ///
    /// 根据启用的编译特性自动选择传输协议。
    pub fn new(config: ServerConfig) -> Self {
        #[cfg(feature = "use-yamux")]
        return Self::with_yamux(config);

        #[cfg(feature = "use-xtransport")]
        return Self::with_xtransport(config);

        #[cfg(not(any(feature = "use-yamux", feature = "use-xtransport")))]
        panic!("Either use-yamux or use-xtransport feature must be enabled");
    }

    /// 使用 Yamux 创建服务器
    #[cfg(feature = "use-yamux")]
    pub fn with_yamux(config: ServerConfig) -> Self {
        Self {
            config,
            listener: None,
            listening: false,
        }
    }

    /// 使用 XTransport 创建服务器
    #[cfg(feature = "use-xtransport")]
    pub fn with_xtransport(config: ServerConfig) -> Self {
        Self {
            config,
            listener: None,
            listening: false,
        }
    }

    /// 启动监听
    pub async fn listen(&mut self) -> Result<()> {
        info!(
            "VirgeServer listening on cid={}, port={}",
            self.config.listen_cid,
            self.config.listen_port
        );

        #[cfg(feature = "use-yamux")]
        {
            let addr = tokio_vsock::VsockAddr::new(self.config.listen_cid, self.config.listen_port);
            let listener = tokio_vsock::VsockListener::bind(addr)
                .map_err(|e| crate::error::VirgeError::ConnectionError(format!("Failed to bind yamux listener: {}", e)))?;
            self.listener = Some(Listener::Yamux(listener));
            self.listening = true;
            return Ok(());
        }

        #[cfg(feature = "use-xtransport")]
        {
            let addr = vsock::VsockAddr::new(self.config.listen_cid, self.config.listen_port);
            let listener = vsock::VsockListener::bind(&addr)
                .map_err(|e| crate::error::VirgeError::ConnectionError(format!("Failed to bind xtransport listener: {}", e)))?;
            self.listener = Some(Listener::XTransport(listener));
            self.listening = true;
            return Ok(());
        }
    }

    /// 接受新的客户端连接
    pub async fn accept(&mut self) -> Result<Box<dyn Transport>> {
        if !self.listening {
            return Err(crate::error::VirgeError::Other(
                "Server not listening".to_string(),
            ));
        }

        #[cfg(feature = "use-yamux")]
        {
            if let Some(Listener::Yamux(listener)) = &mut self.listener {
                let (stream, addr) = listener.accept().await
                    .map_err(|e| crate::error::VirgeError::ConnectionError(format!("Failed to accept yamux connection: {}", e)))?;
                info!("Accepted yamux connection from {:?}", addr);

                // 创建 YamuxTransport 实例并从流初始化
                let mut transport = Box::new(crate::transport::YamuxTransport::new_server());
                transport.from_tokio_stream(stream).await?;

                return Ok(transport);
            } else {
                return Err(crate::error::VirgeError::Other("Yamux listener not initialized".to_string()));
            }
        }

        #[cfg(feature = "use-xtransport")]
        {
            if let Some(Listener::XTransport(listener)) = &mut self.listener {
                let (stream, addr) = listener.accept()
                    .map_err(|e| crate::error::VirgeError::ConnectionError(format!("Failed to accept xtransport connection: {}", e)))?;
                info!("Accepted xtransport connection from {:?}", addr);

                // 创建 XTransportHandler 实例并从流初始化
                let mut transport = Box::new(crate::transport::XTransportHandler::new());
                transport.from_stream(stream).await?;

                return Ok(transport);
            } else {
                return Err(crate::error::VirgeError::Other("XTransport listener not initialized".to_string()));
            }
        }
    }

    /// 停止监听
    pub async fn stop(&mut self) -> Result<()> {
        info!("VirgeServer stopping");

        // 清理监听器
        self.listener = None;

        self.listening = false;
        Ok(())
    }

    /// 检查监听状态
    pub fn is_listening(&self) -> bool {
        self.listening
    }
}
