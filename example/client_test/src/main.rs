use std::io::{Read, Write};

use virga::client::{VirgeClient, ClientConfig};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let config = ClientConfig::new(2, 1234, 1024, false);
    let mut client = VirgeClient::new(config);
    client.connect()?;

    // // 处理发送数据, 先发送数据长度，然后发送数据
    // let data = vec![1; 512];
    // client.write(&data.len().to_be_bytes())?;
    // client.write(&data)?;


    // 处理接收数据, 先接收数据长度，然后创建一个足够长的databuf，最后接收数据
    let mut buf = [0u8; 8];
    client.read_exact(&mut buf)?;
    let data_len = usize::from_be_bytes(buf);
    
    let mut data = vec![0; data_len];
    client.read_exact(&mut data)?;
    
    println!("len date = {}", data_len);
    
    // 断开连接
    //client.disconnect()?;

    Ok(())
}
