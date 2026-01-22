use std::io::{Read, Write};
use std::io::{Error, ErrorKind, Result};
use crate::transport::XTransportHandler;


/// Virga 服务器连接：与VirgeClient类似，负责单个连接的数据传输。
pub struct VirgeServer {
    transport_handler: XTransportHandler, 
    connected: bool,
}

impl VirgeServer {
    pub fn new(trans: XTransportHandler, conn: bool) -> Self{
        Self { transport_handler: trans, connected: conn }
    }
}

impl VirgeServer {
    /// 发送数据
    pub fn send(&mut self, data: Vec<u8>) -> Result<usize> {
        if !self.connected {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Server not connected",
            ));
        }
        self.transport_handler.send(&data)
        .map_err(|e| Error::other(format!("send error: {}", e)))
    }

    /// 接收数据
    pub fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Server not connected",
            ));
        }
        self.transport_handler.recv()
        .map_err(|e| Error::other(format!("send error: {}", e)))
    }

    /// 断开连接
    pub fn disconnect(&mut self) -> Result<()> {
        if self.connected {
            self.transport_handler.disconnect()?;
            self.connected = false;
        }
        Ok(())
    }

    /// 检查连接状态
    pub fn is_connected(&self) -> bool {
        self.connected && self.transport_handler.is_connected()
    }
}


impl Read for VirgeServer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.connected {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Server not connected",
            ));
        }

        let data = self.transport_handler.recv()?;
        let len = std::cmp::min(data.len(), buf.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok(len)
    }

}

impl Write for VirgeServer {
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


