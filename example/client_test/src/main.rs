use virga::client::{VirgeClient, ClientConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let config = ClientConfig::new(2, 1234, 1024, false);
    let mut client = VirgeClient::new(config);
    client.connect()?;
    
    client.send(vec![1; 512])?;
    let data = client.recv()?;
    println!("{}", data.len());
    
    client.disconnect()?;
    Ok(())
}
