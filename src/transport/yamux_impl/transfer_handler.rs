
use std::thread;
use std::thread::JoinHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::error::{Result, VirgeError};
use futures::AsyncReadExt;
use futures::AsyncWriteExt;
use futures::future::poll_fn;
//use futures::executor::block_on;
use log::*;
use tokio_util::compat::Compat;
use tokio_util::compat::TokioAsyncReadCompatExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_vsock::{VsockAddr, VsockStream};

use yamux::Stream;
use yamux::{Config, Connection, Mode};

use std::sync::OnceLock;
use tokio::runtime::Runtime;
static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();
fn get_tokio_runtime() -> &'static Runtime {
    TOKIO_RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create tokio runtime")
    })
}


/// Yamux 传输协议实现
///
/// 直接管理 tokio-vsock 连接并使用 yamux 进行多路复用。
/// Yamux需要持续的驱动程序来处理入站流和连接生命周期。
pub struct YamuxTransportHandler {
    yamux_stream: Option<Stream>,
    connection: Option<Arc<Mutex<Connection<Compat<VsockStream>>>>>,
    driver_handle: Option<JoinHandle<()>>,
    driver_stop_flag: Arc<AtomicBool>,
    mode: Mode,
}

impl YamuxTransportHandler {
    /// 创建客户端模式的 Yamux 传输实例
    pub fn new(mode: Mode) -> Self {
        Self {
            connection: None,
            yamux_stream: None,
            driver_handle: None,
            driver_stop_flag: Arc::new(AtomicBool::new(false)),
            mode,
        }
    }

    /// 获取或创建 yamux 虚拟流
    fn get_or_create_stream(&mut self) -> Result<&mut Stream> {
        if self.yamux_stream.is_some(){
            return Ok(self.yamux_stream.as_mut().unwrap());
        }

        let conn_clone = if let Some(conn) = self.connection.clone() {
            conn
        } else {
            return Err(VirgeError::TransportError(
                "Yamux not initialized".to_string(),
            ));
        };

        match self.mode {
            Mode::Client => {
                let stream = get_tokio_runtime().block_on( async {
                    let mut conn_guard = conn_clone.lock().await;
                    poll_fn(|cx| conn_guard.poll_new_outbound(cx)).await
                });
                match stream {
                    Ok(yamux_stream) => {
                        self.yamux_stream = Some(yamux_stream);
                    }
                    Err(e) => {
                        return Err(VirgeError::TransportError(format!(
                            "Failed to open yamux stream: {}",
                            e
                        )));
                    }
                }
            }
            Mode::Server => {
                let stream = get_tokio_runtime().block_on( async {
                    let mut conn_guard = conn_clone.lock().await;
                    poll_fn(|cx| conn_guard.poll_next_inbound(cx)).await
                });
                match stream {
                    Some(Ok(yamux_stream)) => {
                        self.yamux_stream = Some(yamux_stream);
                    }
                    Some(Err(e)) => {
                        return Err(VirgeError::TransportError(format!(
                            "Failed to open yamux stream: {}",
                            e
                        )));
                    }
                    None => {
                        return Err(VirgeError::TransportError(
                            "Failed to open yamux stream".to_string(),
                        ));
                    }
                }

            }
        }

        Ok(self.yamux_stream.as_mut().unwrap())
       
    }

    /// yamux 连接驱动程序
    ///
    /// 处理连接事件（需要在后台线程运行）
    fn start_driver(&mut self) -> Result<()> {
        if self.connection.is_none(){
            return Err(VirgeError::ConnectionError("connection is none".to_string()));
        }

        let conn_clone = self.connection.clone().unwrap();
        let stop_flag_clone = Arc::clone(&self.driver_stop_flag);
        let driver_handle: JoinHandle<()> = thread::spawn(move || {
            debug!("Starting yamux connection driver");
            loop {
                // 检查停止标志
                if stop_flag_clone.load(Ordering::Relaxed) {
                    debug!("Yamux connection driver received stop signal");
                    break;
                }

                let should_break = get_tokio_runtime().block_on(async {
                    let mut conn_guard = conn_clone.lock().await;
                    match poll_fn(|cx| conn_guard.poll_next_inbound(cx)).await {
                        Some(Ok(_)) => false,  // 继续循环
                        Some(Err(e)) => {
                            warn!("Yamux connection error: {}", e);
                            true  // 跳出循环
                        }
                        None => {
                            warn!("Yamux connection closed");
                            true  // 跳出循环
                        }
                    }
                });

                if should_break {
                    break;
                }
            }

            info!("Yamux connection driver stopped");
        });

        self.driver_handle = Some(driver_handle);

        Ok(())
    }

}


impl YamuxTransportHandler {
    pub fn connect(&mut self, cid: u32, port: u32, _: u32, _: bool) -> Result<()> {
        info!("Yamux transport connecting to cid={}, port={}", cid, port);
        
        // 使用全局运行时
        let stream = get_tokio_runtime().block_on(async {
            VsockStream::connect(VsockAddr::new(cid, port)).await
        }).map_err(|e| VirgeError::ConnectionError(format!("Failed to connect vsock: {}", e)))?;

        // 初始化 yamux
        let config = Config::default();
        let connection = Connection::new(stream.compat(), config, self.mode);
        self.connection = Some(Arc::new(Mutex::new(connection)));

        // 启动驱动程序来处理连接生命周期
        let _ = self.start_driver();

        // 创建yamux_stream
        let _ = self.get_or_create_stream()?;

        info!("Yamux transport connected successfully");
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        info!("Yamux transport disconnecting");

        // 设置停止标志，让驱动线程退出
        self.driver_stop_flag.store(true, Ordering::Relaxed);

        // 等待驱动线程结束
        if let Some(handle) = self.driver_handle.take() {
            let _ = handle.join();
        }

        // 清理资源
        self.connection = None;
        self.yamux_stream = None;

        info!("Yamux transport disconnected");
        Ok(())
    }

    pub fn send(&mut self, data: &[u8]) -> Result<usize> {
        if !self.is_connected() {
            return Err(VirgeError::TransportError(
                "Yamux transport not connected about send".to_string(),
            ));
        }
        println!("send len is {}", data.len());

        let stream = self.get_or_create_stream()?;
        get_tokio_runtime().block_on( async {
            stream.write_all(&data).await.map_err(|e| VirgeError::Other(format!("yamux send error: {}", e)))?;
            // stream.close().await?;
            Ok::<_, std::io::Error>(())
        })?;

        info!("Yamux sent {} bytes", data.len());
        Ok(data.len())
    }

    pub fn recv(&mut self) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(VirgeError::TransportError(
                "Yamux transport not connected about recv".to_string(),
            ));
        }
        let stream = self.get_or_create_stream()?;
        let mut buf = Vec::new();
        get_tokio_runtime().block_on(async {
            stream.read_to_end(&mut buf).await.map_err(|e| VirgeError::Other(format!("yamux recv error: {}", e)))?;
            Ok::<_, std::io::Error>(())
        })?;
        info!("Yamux received {} bytes", buf.len());
        Ok(buf)
    }

    pub fn is_connected(&self) -> bool {
        self.yamux_stream.is_some() && self.connection.is_some()
    }

    pub fn from_tokio_stream(&mut self, stream: VsockStream) -> Result<()> {
        // 初始化 yamux
        let config = Config::default();
        let connection = Connection::new(stream.compat(), config, self.mode);

        self.connection = Some(Arc::new(Mutex::new(connection)));

        // 启动驱动程序来处理连接生命周期
        let _ = self.start_driver();
        // 创建yamux_stream
        let _ = self.get_or_create_stream()?;

        info!("Yamux transport initialized from stream successfully");
        Ok(())
    }
}
