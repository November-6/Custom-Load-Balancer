use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::args().nth(1).unwrap_or("9000".to_string());
    let addr = format!("127.0.0.1:{}", port);

    let listener = TcpListener::bind(&addr).await?;
    println!("Backend listening on {}", addr);


// I have a TCP listener that accepts connections and gives me a socket and peer.
// When I accept a connection, I want to handle it without blocking the accept loop, so I spawn a new async task using tokio::spawn.
// This task runs concurrently with other tasks, and Tokio schedules it on available worker threads.
// The async move block transfers ownership of only the variables used inside the task (like socket) so the task can safely outlive the current scope without ownership or lifetime issues.

    loop {
        let (mut socket, peer) = listener.accept().await?;
        println!("Accepted connection from {}", peer);

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];

            let n = match socket.read(&mut buf).await {
                Ok(0) => return,
                Ok(n) => n,
                Err(_) => return,
            };

            if buf[..n] == *b"HEALTH\n".as_slice() {
                let _ = socket.write_all(b"OK\n").await;
                return;
            }

            let _ = socket.write_all(&buf[..n]).await;

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(_) => return,
                };

                if socket.write_all(&buf[..n]).await.is_err() {
                    return;
                } 
            }
        });
    }
}
