use tokio::net::{TcpListener, TcpStream};
use tokio::io::copy_bidirectional;
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Load balancer listening on 127.0.0.1:8080");

    let backends = Arc::new(vec![
        "127.0.0.1:9000".to_string(),
        "127.0.0.1:9001".to_string(),
    ]);

    let rr_counter = Arc::new(AtomicUsize::new(0));

    loop {
        let (client, client_addr) = listener.accept().await?;
        println!("Client connected from {}", client_addr);

        let backends = Arc::clone(&backends);
        let rr_counter = Arc::clone(&rr_counter);

        tokio::spawn(async move {
            if let Err(e) =
                handle_connection(client, client_addr, backends, rr_counter).await
            {
                eprintln!("Connection error [{}]: {}", client_addr, e);
            }
        });
    }
}

async fn handle_connection(
    mut client: TcpStream,
    client_addr: SocketAddr,
    backends: Arc<Vec<String>>,
    rr_counter: Arc<AtomicUsize>,
) -> std::io::Result<()> {
    let index = rr_counter.fetch_add(1, Ordering::Relaxed) % backends.len();
    let backend_addr = &backends[index];

    let mut backend = TcpStream::connect(backend_addr).await?;
    println!("Routing {} → {}", client_addr, backend_addr);

    client.set_nodelay(true)?;
    backend.set_nodelay(true)?;

    let (c2b, b2c) = copy_bidirectional(&mut client, &mut backend).await?;

    println!(
        "Closed {} → {} | c→b {} bytes | b→c {} bytes",
        client_addr, backend_addr, c2b, b2c
    );

    Ok(())
}
