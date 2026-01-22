//! 错误定义模块
//!
//! 统一定义库中所有的错误类型，使用 `thiserror` 或自定义 enum。
//!
//! # 错误分类
//! - `ConnectionError`：vsock 连接相关错误（连接失败、超时等）
//! - `TransportError`：传输协议相关错误（编码、解码、发送、接收失败）
//! - `InvalidConfig`：配置参数非法
//! - `Unknown`：未知错误

use std::fmt;

/// 库的统一错误类型
#[derive(Debug)]
pub enum VirgeError {
    /// 连接层错误
    ConnectionError(String),

    /// 传输层错误
    TransportError(String),

    /// 配置错误
    ConfigError(String),

    /// IO 错误
    IoError(std::io::Error),

    /// 其他错误
    Other(String),
}

impl fmt::Display for VirgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VirgeError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            VirgeError::TransportError(msg) => write!(f, "Transport error: {}", msg),
            VirgeError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            VirgeError::IoError(e) => write!(f, "IO error: {}", e),
            VirgeError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for VirgeError {}

impl From<std::io::Error> for VirgeError {
    fn from(err: std::io::Error) -> Self {
        VirgeError::IoError(err)
    }
}

impl From<VirgeError> for std::io::Error {
    fn from(err: VirgeError) -> Self {
        match err {
            VirgeError::IoError(e) => e,
            VirgeError::ConnectionError(msg) => std::io::Error::new(std::io::ErrorKind::ConnectionRefused, msg),
            VirgeError::TransportError(msg) => std::io::Error::new(std::io::ErrorKind::InvalidData, msg),
            VirgeError::ConfigError(msg) => std::io::Error::new(std::io::ErrorKind::InvalidInput, msg),
            VirgeError::Other(msg) => std::io::Error::new(std::io::ErrorKind::Other, msg),
        }
    }
}

#[cfg(feature = "use-xtransport")]
impl From<xtransport::Error> for VirgeError {
    fn from(err: xtransport::Error) -> Self {
        VirgeError::Other(format!("XTransport error: {}", err))
    }
}

/// 操作结果类型别名
pub type Result<T> = std::result::Result<T, VirgeError>;
