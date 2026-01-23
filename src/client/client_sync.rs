use std::io::{Read, Write};
use std::io::{Error, ErrorKind, Result};

use log::*;

use super::ClientConfig;
use crate::ReadState;
use crate::transport::XTransportHandler;


/// 同步客户端
pub struct VirgeClient {
    transport_handler: XTransportHandler,
    config: ClientConfig,
    connected: bool,
    read_buffer: Vec<u8>,  // 读取缓存
    read_state: ReadState,  // 读取状态
}

impl VirgeClient {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            transport_handler: XTransportHandler::new(),
            config,
            connected: false,
            read_buffer: Vec::new(),
            read_state: ReadState::Idle,
        }
    }

    /// 建立连接
    pub fn connect(&mut self) -> Result<()> {
        info!(
            "VirgeClient connecting to cid={}, port={}",
            self.config.server_cid, self.config.server_port
        );

        self.transport_handler
            .connect(
                self.config.server_cid,
                self.config.server_port,
                self.config.chunk_size,
                self.config.is_ack,
            )?;
        self.connected = true;
        Ok(())
    }

    /// 断开连接
    pub fn disconnect(&mut self) -> Result<()> {
        info!("VirgeClient disconnecting");
        if !self.read_buffer.is_empty() {
            warn!("Disconnecting with {} bytes of unread data in buffer", self.read_buffer.len());
            return Err(Error::new(
                ErrorKind::Other,
                format!("Cannot disconnect: {} bytes of unread data remaining", self.read_buffer.len()),
            ));
        }

        self.transport_handler.disconnect()?;
        self.connected = false;
        Ok(())
    }

    /// 发送数据
    pub fn send(&mut self, data: Vec<u8>) -> Result<usize> {
        if !self.connected {
            return Err(Error::new(
                ErrorKind::NotConnected, 
                format!("Client not connected"),
                )
            );
        }

        self.transport_handler.send(&data)
        .map_err(|e| Error::other(format!("send error: {}", e)))
    }

    /// 接收数据
    pub fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(Error::new(
                ErrorKind::NotConnected, 
                format!("Client not connected"),
                )
            );
        }

        self.transport_handler.recv()
        .map_err(|e| Error::other(format!("recv error: {}", e)))
    }

    /// 检查连接状态
    pub fn is_connected(&self) -> bool {
        self.connected && self.transport_handler.is_connected()
    }
}

impl VirgeClient {
    fn read_new_message(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.transport_handler.recv() {
            Ok(data) => {
                if data.len() <= buf.len() {
                    buf[..data.len()].copy_from_slice(&data);
                    Ok(data.len())
                } else {
                    let len = buf.len();
                    buf.copy_from_slice(&data[..len]);
                    self.read_buffer.extend_from_slice(&data[len..]);
                    
                    self.read_state = ReadState::Reading {
                        total: data.len(),
                        read: len,
                    };
                    Ok(len)
                }
            }
            Err(e) => return Err(Error::new(
                ErrorKind::Other,
                format!("Read error: {}", e),
            )),
        }
    }

    /// 检查是否还有数据可读（包括rbuf中的数据）
    pub fn no_has_data(&self) -> bool {
        self.read_buffer.is_empty() && self.read_state == ReadState::Idle
    }
    
}

impl Read for VirgeClient {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.connected {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Client not connected",
            ));
        }

        match self.read_state {
            ReadState::Idle => {
                // 直接从传输层读取
                return self.read_new_message(buf);
            }
            ReadState::Reading { total, read, .. } => {
                // 从rbuf中读取剩余数据
                if !self.read_buffer.is_empty() {
                    let len = std::cmp::min(self.read_buffer.len(), buf.len());
                    buf[..len].copy_from_slice(&self.read_buffer[..len]);
                    self.read_buffer.drain(..len);
                    
                    let new_read = read + len;
                    if new_read == total {
                        // 消息读取完成
                        self.read_state = ReadState::Idle;
                    } else {
                        // 更新状态
                        self.read_state = ReadState::Reading {
                            total,
                            read: new_read,
                        };
                    }
                    Ok(len)
                } else {
                    // rbuf为空但状态是Reading，这不应该发生
                    self.read_state = ReadState::Idle;
                    Ok(0)
                }
            }
        }
    }
}

impl Write for VirgeClient {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if !self.connected {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Client not connected",
            ));
        }

        match self.transport_handler.send(buf) {
            Ok(len) => Ok(len),
            Err(e) => Err(Error::new(
                ErrorKind::Other,
                format!("Write error: {}", e),
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}


