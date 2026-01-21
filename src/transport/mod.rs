//! 传输协议层模块
//!
//! 定义和实现各种传输协议（yamux、xtransport 等）。
//!
//! # 职责
//! - 定义统一的 Transport trait
//! - 实现具体的传输协议
//! - 直接管理 vsock 连接和协议逻辑
//! - 提供开箱即用的 connect/disconnect/send/recv 接口
//!

#[cfg(feature = "use-xtransport")]
pub mod xtransport_impl;
#[cfg(feature = "use-yamux")]
pub mod yamux_impl;

use crate::error::Result;
use async_trait::async_trait;

/// 传输协议抽象 trait
#[async_trait]
pub trait Transport: Send + Sync {
    /// 建立 vsock 连接并初始化传输协议（客户端模式）
    ///
    /// # Arguments
    /// - `cid`: vsock 连接标识符
    /// - `port`: vsock 端口号
    ///
    /// # Returns
    /// 连接成功返回 Ok，否则返回错误
    async fn connect(&mut self, cid: u32, port: u32, chunksize: u32, isack: bool) -> Result<()>;

    /// 从现有 vsock 流初始化传输协议（服务器模式）
    ///
    /// # Arguments
    /// - `stream`: 已建立的 vsock 连接流
    ///
    /// # Returns
    /// 初始化成功返回 Ok，否则返回错误
    #[cfg(feature = "use-yamux")]
    async fn from_tokio_stream(&mut self, _stream: tokio_vsock::VsockStream) -> Result<()> {
        Err(crate::error::VirgeError::Other(
            "Yamux from_tokio_stream not implemented".to_string(),
        ))
    }

    #[cfg(feature = "use-xtransport")]
    async fn from_stream(
        &mut self,
        _stream: vsock::VsockStream,
        _chunksize: u32,
        _isack: bool,
    ) -> Result<()> {
        Err(crate::error::VirgeError::Other(
            "XTransport from_stream not implemented".to_string(),
        ))
    }

    /// 断开连接并清理资源
    async fn disconnect(&mut self) -> Result<()>;

    /// 发送数据
    ///
    /// # Arguments
    /// - `data`: 要发送的字节数据
    ///
    /// # Returns
    /// 成功发送返回 Ok，否则返回错误
    async fn send(&mut self, data: Vec<u8>) -> Result<()>;

    /// 接收数据
    ///
    /// # Returns
    /// 返回接收到的字节数据，或错误
    async fn recv(&mut self) -> Result<Vec<u8>>;

    /// 检查连接是否活跃
    fn is_connected(&self) -> bool;
}

// 具体实现模块
#[cfg(feature = "use-xtransport")]
pub use xtransport_impl::XTransportHandler;
#[cfg(feature = "use-yamux")]
pub use yamux_impl::YamuxTransport;
