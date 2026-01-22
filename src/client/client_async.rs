use std::io::{Read, Write};
use std::io::{Error, ErrorKind, Result};

use log::*;

use super::ClientConfig;
use crate::transport::YamuxTransportHandler;


/// 异步客户端，但内部已经同步化了
pub struct VirgeClient {
    transport_handler: YamuxTransportHandler,
    config: ClientConfig,
    connected: bool,
}

impl VirgeClient {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            transport_handler: YamuxTransportHandler::new(yamux::Mode::Client),
            config,
            connected: false,
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
        self.transport_handler.disconnect()?;
        self.connected = false;
        Ok(())
    }

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

