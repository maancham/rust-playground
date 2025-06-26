use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream_result in listener.incoming() {
        let stream = stream_result.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0; 1024];
    let read_bufsize = stream.read(&mut buf).expect("failed to read from client");

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
        .expect("failed to write to client");
}
