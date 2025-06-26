use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            handle_connection(stream).await
        });
    }
}

async fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0; 1024];
    let read_bufsize = stream.read(&mut buf).await.unwrap();

    let request = String::from_utf8_lossy(&buf[0..read_bufsize]);
    let lines: Vec<&str> = request.lines().collect();

    let first_line = lines.first().unwrap();
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return;
    }

    let path = parts[1];

    let response = match path {
        "/" => "HTTP/1.1 200 OK\r\n\r\n",
        path if path.starts_with("/echo/") => {
            if let Some(post_path) = path.strip_prefix("/echo/") {
                &format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    post_path.len(),
                    post_path
                )
            } else {
                "HTTP/1.1 404 Not Found\r\n\r\n"
            }
        }
        "/user-agent" => {
            let agent = lines.iter()
                .find(|line| line.starts_with("User-Agent"))
                .map(|line| line.split(":").nth(1).unwrap().trim()).unwrap();
            
            &format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                agent.len(),
                agent
            )
        }
        _ => "HTTP/1.1 404 Not Found\r\n\r\n",
    };

    stream
        .write_all(response.as_bytes())
        .await.unwrap();
}
