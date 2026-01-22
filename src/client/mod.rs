
//! 客户端模块
//!
//! 提供客户端角色的高级 API。
//!
//! # 职责
//! - 封装客户端的连接逻辑
//! - 提供简洁的发送/接收接口
//! - 管理传输协议选择

#[cfg(feature = "use-xtransport")]
pub mod client_sync;
#[cfg(feature = "use-xtransport")]
pub use client_sync::VirgeClient;

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