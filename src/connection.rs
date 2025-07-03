use std::io::{Error, ErrorKind};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use httparse;

use crate::utils::RequestInfo;
use crate::router::route_request;

pub async fn handle_connection(mut stream: TcpStream, file_directory: String) -> Result<(), std::io::Error> {
    loop {
        let mut buf = [0; 1024];
        let read_bufsize = stream.read(&mut buf).await.unwrap();

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let request_result = req.parse(&buf[0..read_bufsize]);

        match request_result {
            Ok(httparse::Status::Complete(headers_end)) => {
                let method = req.method.unwrap();
                let path = req.path.unwrap();

                let request_info = RequestInfo::from_headers(&req.headers);
                let body = read_body(&mut stream, &buf[headers_end..read_bufsize], request_info.content_length).await;
                let response = route_request(method, path, &request_info, &body, &file_directory).await;
                stream.write_all(&response).await.unwrap();

                if request_info.close_connection {
                    return Err(Error::new(ErrorKind::Other, "client requested close connection"));
                }
            }
            Ok(httparse::Status::Partial) => {
                println!("received partial request");
                break;
            },
            Err(e) => {
                println!("failed to parse request: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn read_body(stream: &mut TcpStream, initial_body: &[u8], content_length: Option<usize>) -> Vec<u8> {
    let mut body = initial_body.to_vec();
    if let Some(content_len) = content_length {
        let remaining_bytes = content_len.saturating_sub(body.len());
        if remaining_bytes > 0 {
            let mut remaining_buf = vec![0; remaining_bytes];
            stream.read_exact(&mut remaining_buf).await.unwrap();
            body.extend_from_slice(&remaining_buf);
        }
    } 
    body
} 
