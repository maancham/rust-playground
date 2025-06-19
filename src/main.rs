#[allow(unused_imports)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {    
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream_result in listener.incoming() {
        let stream= stream_result.unwrap();
        println!("Connection established!");
        handle_client(stream);
    }
}


fn handle_client(mut stream: TcpStream) {
    let mut buf = [0; 512];
    let _read_bufsize = stream.read(&mut buf).expect("failed to read from client");

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).expect("failed to write to client");    
}
