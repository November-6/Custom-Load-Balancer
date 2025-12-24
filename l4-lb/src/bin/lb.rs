use tokio::net::{TcpListener, TcpStream};
use tokio::io::{copy_bidirectional, AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::time::{sleep, Duration};

#[derive(Debug)]
struct Backend {
    addr: String,
    healthy: AtomicBool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Load balancer listening on 127.0.0.1:8080");

    let backends = Arc::new(vec![
        Backend {
            addr: "127.0.0.1:9000".to_string(),
            healthy: AtomicBool::new(true),
        },
        Backend {
            addr: "127.0.0.1:9001".to_string(),
            healthy: AtomicBool::new(true),
        },
        Backend {
            addr: "127.0.0.1:9002".to_string(),
            healthy: AtomicBool::new(true),
        },
    ]);

    let rr_counter = Arc::new(AtomicUsize::new(0));

    start_health_checker(Arc::clone(&backends));

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

fn start_health_checker(backends: Arc<Vec<Backend>>) {
    tokio::spawn(async move {
        loop {
            for backend in backends.iter() {
                let healthy = check_backend_health(&backend.addr).await;
                let prev = backend.healthy.swap(healthy, Ordering::Relaxed);

                if prev != healthy {
                    println!(
                        "----------------------------------Health change: {} → {}",
                        backend.addr,
                        if healthy { "UP" } else { "DOWN" }
                    );
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    });
}

async fn check_backend_health(addr: &str) -> bool {
    match TcpStream::connect(addr).await {
        Ok(mut stream) => {
            if stream.write_all(b"HEALTH\n").await.is_err() {
                return false;
            }

            let mut buf = [0u8; 16];
            match stream.read(&mut buf).await {
                Ok(n) => &buf[..n] == b"OK\n",
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

async fn handle_connection(
    mut client: TcpStream,
    client_addr: SocketAddr,
    backends: Arc<Vec<Backend>>,
    rr_counter: Arc<AtomicUsize>,
) -> std::io::Result<()> {
    let total = backends.len();

    for _ in 0..total {
        let index = rr_counter.fetch_add(1, Ordering::Relaxed) % total;
        let backend = &backends[index];

        if !backend.healthy.load(Ordering::Relaxed) {
            continue;
        }

        match TcpStream::connect(&backend.addr).await {
            Ok(mut backend_stream) => {
                println!("Routing {} → {}", client_addr, backend.addr);

                client.set_nodelay(true)?;
                backend_stream.set_nodelay(true)?;

                let (c2b, b2c) =
                    copy_bidirectional(&mut client, &mut backend_stream).await?;

                println!(
                    "Closed {} → {} | c→b {} bytes | b→c {} bytes",
                    client_addr, backend.addr, c2b, b2c
                );
                return Ok(());
            }
            Err(_) => {
                backend.healthy.store(false, Ordering::Relaxed);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "No healthy backends available",
    ))
}

// found this really cool saying: 
// if you desire only what depends on you, no one can ever control you