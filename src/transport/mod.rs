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
mod xtransport_impl;

// 具体实现模块
#[cfg(feature = "use-xtransport")]
pub use xtransport::transport;
pub use xtransport_impl::XTransportHandler;
