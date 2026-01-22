//! Yamux 传输协议实现
//!
//! 基于 yamux 库的多路复用传输实现。
//!
//! # 特点
//! - 支持多个独立的虚拟流
//! - 适合多并发场景
//! - 由 libp2p 社区维护
//!

mod transfer_handler;
pub use transfer_handler::YamuxTransportHandler;
