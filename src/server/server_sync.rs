use std::io::{Read, Write};
use std::io::{Error, ErrorKind, Result};
use log::*;
use crate::transport::XTransportHandler;


/// Virga 服务器连接：与VirgeClient类似，负责单个连接的数据传输。
pub struct VirgeServer {
    transport_handler: XTransportHandler, 
    connected: bool,
    read_buffer: Vec<u8>,  // 读取缓存
    read_total_len: usize, // 读取消息总长度
}

impl VirgeServer {
    pub fn new(trans: XTransportHandler, conn: bool) -> Self{
        Self { 
            transport_handler: trans, 
            connected: conn,
            read_buffer: Vec::new(),
            read_total_len: 0,
        }
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
        info!("VirgeServer disconnecting");
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
        
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(self.read_buffer.len(), buf.len());
            buf[..len].copy_from_slice(&self.read_buffer[..len]);
            self.read_buffer.drain(..len);
            if self.read_buffer.is_empty(){
                self.read_total_len = 0;
            }
            return Ok(len);
        }
        // 之前读到的消息数据已全部被拿走
        if self.read_buffer.is_empty() && self.read_total_len != 0{
            self.read_total_len = 0;
            return Ok(0);
        }

        match self.transport_handler.recv() {
            Ok(data) => {
                self.read_total_len = data.len();
                if data.len() <= buf.len() {
                    buf[..data.len()].copy_from_slice(&data);
                    Ok(data.len())
                } else {
                    buf.copy_from_slice(&data[..buf.len()]);
                    self.read_buffer.extend_from_slice(&data[buf.len()..]);
                    Ok(buf.len())
                }
            }
            Err(e) => Err(Error::new(
                ErrorKind::Other,
                format!("Read error: {}", e),
            )),
        }
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


