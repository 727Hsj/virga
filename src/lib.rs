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
//! use virga::server::{VirgeServer, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::default();
//!     let mut server = VirgeServer::with_yamux(config);
//!     server.listen().await?;
//!
//!     while let Ok(mut transport) = server.accept().await {
//!         tokio::spawn(async move {
//!             let data = transport.recv().await?;
//!             transport.send(data).await?;
//!             transport.disconnect().await?;
//!             Ok::<(), Box<dyn std::error::Error>>(())
//!         });
//!     }
//! }
//! ```

// 特性冲突检测
#[cfg(all(feature = "use-yamux", feature = "use-xtransport"))]
compile_error!("Features 'use-yamux' and 'use-xtransport' cannot be enabled at the same time. Please choose only one transport protocol.");

// 错误层
pub mod error;
pub use error::{VirgeError, Result};

// 协议层
pub mod transport;

// 应用层
pub mod client;
pub mod server;

pub use client::{VirgeClient, ClientConfig};
pub use server::{VirgeServer, ServerConfig};


pub const DEFAULT_SERVER_CID: usize = 103;
pub const VMADDR_CID_ANY: usize = 0xFFFFFFFF;
pub const DEFAULT_SERVER_PORT: usize = 1234;

pub const KIB: usize = 1024;
pub const MIB: usize = KIB * 1024;
pub const GIB: usize = MIB * 1024;

