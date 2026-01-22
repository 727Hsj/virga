use virga::server::{ServerManager, ServerConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = ServerConfig::new(0xFFFFFFFF, 1234, 1024, false);

    let mut manager = ServerManager::new(config);
    manager.start()?;

    while let Ok(mut server) = manager.accept() {
        println!("there is a new virgeserver");
        tokio::spawn(async move {
            // 处理接收数据
            if server.is_connected(){
                println!("after get virga server, the server is connected");
            }
            let data_result = server.recv();
            println!("server.recv");
            let data = match data_result {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("接收数据失败: {}", e);
                    return;  // 直接返回，不继续执行
                }
            };
            println!("len date = {}", data.len());
            
            // 处理发送数据
            if let Err(e) = server.send(data) {
                eprintln!("发送数据失败: {}", e);
            }
            
            // 处理断开连接
            if let Err(e) = server.disconnect() {
                eprintln!("断开连接失败: {}", e);
            }
        });
    }

    Ok(())
}