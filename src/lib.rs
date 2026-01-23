//! Virga: 基于 vsock 的传输库
//!
//! Virga 是一个基于 vsock 的字节流传输库，支持多种传输协议（yamux、xtransport 等）。
//!
//! # 架构分层
//! - **应用层（Application）**：`VirgeClient`、`VirgeServer` - 用户直接使用的高级 API
//! - **协议层（Protocol）**：`Transport` trait 及其实现（Yamux、XTransport）- 直接管理 vsock 连接
//! - **错误层（Error）**：统一的错误类型
//!
//! # 快速开始
//!
//! ## 客户端使用
//! ```ignore
//! use virga::client::{VirgeClient, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ClientConfig::default();
//!     let mut client = VirgeClient::with_yamux(config);
//!     client.connect().await?;
//!     
//!     client.send(vec![1, 2, 3, 4, 5]).await?;
//!     let data = client.recv().await?;
//!     
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## 服务器使用
//! ```ignore
//! use virga::server::{ServerManager, VirgeServer, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::default();
//!
//!     // 方法1：简单回显服务器（自动管理连接）
//!     let manager = ServerManager::new(config);
//!     manager.run_simple().await?;
//!
//!     // 方法2：自定义连接处理器
//!     let manager = ServerManager::new(config);
//!     manager.run(|mut server| async move {
//!         // 处理每个VirgeServer连接的业务逻辑
//!         let data = server.recv().await?;
//!         server.send(data).await?;
//!         server.disconnect().await?;
//!     }).await?;
//!
//!     // 方法3：手动管理连接
//!     let mut manager = ServerManager::new(config);
//!     manager.start().await?;
//!
//!     while let Ok(mut server) = manager.accept().await {
//!         tokio::spawn(async move {
//!             let data = server.recv().await?;
//!             server.send(data).await?;
//!             server.disconnect().await?;
//!             Ok::<(), Box<dyn std::error::Error>>(())
//!         });
//!     }
//! }
//! ```

#[cfg(all(feature = "use-xtransport", feature = "use-yamux"))]
compile_error!("feature1 and feature2 cannot be enabled at the same time");

// 错误层
pub mod error;
pub use error::{Result, VirgeError};

// 协议层
pub mod transport;

// 应用层
pub mod client;
pub mod server;

pub use client::{ClientConfig, VirgeClient};
pub use server::{ServerConfig, ServerManager, VirgeServer};

pub const KIB: usize = 1024;
pub const MIB: usize = KIB * 1024;
pub const GIB: usize = MIB * 1024;

pub const DEFAULT_SERVER_CID: usize = 103;
pub const VMADDR_CID_ANY: usize = 0xFFFFFFFF;
pub const DEFAULT_SERVER_PORT: usize = 1234;

pub const DEAFULT_CHUNK_SIZE: usize = KIB;
pub const DEFAULT_IS_ACK: bool = false;

#[derive(PartialEq)]
enum ReadState {
    Idle,                    // 空闲，等待新消息
    Reading {                // 正在读取消息
        total: usize,        // 消息总长度
        read: usize,         // 已读取长度
    },
}