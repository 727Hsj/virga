
use crate::transport::YamuxTransportHandler;

/// Virga 服务器连接：与VirgeClient类似，负责单个连接的数据传输。
pub struct VirgeServer {
    transport_handler: YamuxTransportHandler,
    connected: bool,
}

impl VirgeServer {
    pub fn new(trans: YamuxTransportHandler, conn: bool) -> Self{
        Self { transport_handler: trans, connected: conn }
    }
}

impl VirgeServer {
    /// 发送数据
    pub async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        if !self.connected {
            return Err(VirgeError::TransportError(
                "Server not connected".to_string(),
            ));
        }
        self.transport.send(data).await
    }

    /// 接收数据
    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.connected {
            return Err(VirgeError::TransportError(
                "Server not connected".to_string(),
            ));
        }
        self.transport.recv().await
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        if self.connected {
            self.transport.disconnect().await?;
            self.connected = false;
        }
        Ok(())
    }

    /// 检查连接状态
    pub fn is_connected(&self) -> bool {
        self.connected && self.transport.is_connected()
    }
}
