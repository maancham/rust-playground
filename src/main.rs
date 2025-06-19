#[allow(unused_imports)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {    
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream_result in listener.incoming() {
        let stream= stream_result.unwrap();
        handle_client(stream);
    }
}


fn handle_client(mut stream: TcpStream) {
    let mut buf = [0; 512];
    let read_bufsize = stream.read(&mut buf).expect("failed to read from client");

    let request = String::from_utf8_lossy(&buf[0..read_bufsize]);
    let first_line = request.lines().next().unwrap();
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    let response = match parts[1] {
        "/" => "HTTP/1.1 200 OK\r\n\r\n",
        _ => "HTTP/1.1 404 Not Found\r\n\r\n",
    };

    stream.write_all(response.as_bytes()).expect("failed to write to client");    
}
