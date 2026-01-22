use std::io::{Read, Write};
use virga::server::{ServerManager, ServerConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = ServerConfig::new(0xFFFFFFFF, 1234, 1024, false);

    let mut manager = ServerManager::new(config);
    manager.start()?;

    if let Ok(mut server) = manager.accept() {
        println!("there is a new virgeserver");
        
        // 处理接收数据, 先接收数据长度，然后创建一个足够长的databuf，最后接收数据
        let mut buf: [u8; 8] = [0u8; 8];
        server.read(&mut buf)?;
        let data_len = usize::from_be_bytes(buf);
        println!("data_len: {data_len}");
        
        let mut data = vec![0; data_len];
        let actual_data_len_ = server.read(&mut data)?;
        assert_eq!(data_len, actual_data_len_);
        
        // 处理发送数据, 先发送数据长度，然后发送数据
        server.write_all(&data.len().to_be_bytes())?;
        server.write_all(&data)?;
        
        // 断开连接
        // server.disconnect()?;
    }

    Ok(())
}