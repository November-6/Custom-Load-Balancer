use tokio::net::{TcpListener, TcpStream};
use tokio::io::copy_bidirectional;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Load balancer listening on 8080");

    loop {
        let (mut client, addr) = listener.accept().await?;
        println!("Client connected from {}", addr);

        tokio::spawn(async move {
            let mut backend = match TcpStream::connect("127.0.0.1:9000").await {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Backend connect failed: {}", e);
                    return;
                }
            };

            let _ = copy_bidirectional(&mut client, &mut backend).await;
            println!("Connection closed");
        });
    }
}
