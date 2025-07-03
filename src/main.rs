use std::fs;
use std::io::{Error, ErrorKind, Write};
use std::collections::HashMap;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use clap::Parser;

use flate2::Compression;
use flate2::write::GzEncoder;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "./tmp")]
    directory: String,
}


#[tokio::main]
async fn main() {
    let args = Args::parse();

    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let file_directory = args.directory.clone();
        tokio::spawn(async move {
            if let Err(_) = handle_connection(stream, file_directory).await {
                println!("client closed connection");
                return;
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream, file_directory: String) -> Result<(), std::io::Error> {
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


#[derive(Debug)]
struct RequestInfo {
    content_length: Option<usize>,
    user_agent: Option<String>,
    accepts_gzip: bool,
    close_connection: bool,
}

impl RequestInfo {
    fn from_headers(headers: &[httparse::Header]) -> Self {
        let header_map: HashMap<String, String> = headers
            .iter()
            .filter_map(|h| {
                let name = h.name.to_lowercase();
                let value = std::str::from_utf8(h.value).ok()?.to_string();
                Some((name, value))
            })
            .collect();

        let content_length = header_map
            .get("content-length")
            .and_then(|v| v.parse().ok());

        let user_agent = header_map.get("user-agent").cloned();

        let accepts_gzip = header_map
            .get("accept-encoding")
            .map(|encodings| {
                encodings.split(',')
                    .map(|e| e.trim())
                    .any(|e| e == "gzip")
            })
            .unwrap_or(false);

        let close_connection = header_map
            .get("connection")
            .map(|v| v.to_lowercase() == "close")
            .unwrap_or(false);

        Self {
            content_length,
            user_agent,
            accepts_gzip,
            close_connection,
        }
    }
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

struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponse {
    fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn with_text_body(mut self, content_type: &str, body: &str) -> Self {
        self.headers.push(("Content-Type".to_string(), content_type.to_string()));
        self.body = body.as_bytes().to_vec();
        self
    }

    fn with_gzip_compression(mut self) -> Self {
        if !self.body.is_empty() {
            self.body = gzip_bytes(&self.body);
            self.headers.push(("Content-Encoding".to_string(), "gzip".to_string()));
        }
        self
    }

    fn add_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    fn to_bytes(self) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status,
            status_text(self.status)
        );

        response.push_str(&format!("Content-Length: {}\r\n", self.body.len()));

        for (name, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", name, value));
        }

        response.push_str("\r\n");

        let mut result = response.into_bytes();
        result.extend_from_slice(&self.body);
        result
    }

}

async fn route_request(
    method: &str,
    path: &str,
    request_info: &RequestInfo,
    body: &[u8],
    file_directory: &str,
) -> Vec<u8> {
    let mut response = match (method, path) {
        ("GET", "/") => {
            HttpResponse::new(200)
        }
        ("GET", "/user-agent") => {
            if let Some(agent) = &request_info.user_agent {
                let mut resp = HttpResponse::new(200)
                    .with_text_body("text/plain", agent);
                
                if request_info.accepts_gzip {
                    resp = resp.with_gzip_compression();
                }
                resp
            } else {
                HttpResponse::new(404)
            }
        }
        ("GET", path) if path.starts_with("/echo/") => {
            if let Some(echo_text) = path.strip_prefix("/echo/") {
                let mut resp = HttpResponse::new(200)
                    .with_text_body("text/plain", echo_text);
                
                if request_info.accepts_gzip {
                    resp = resp.with_gzip_compression();
                }
                resp
            } else {
                HttpResponse::new(404)
            }
        }
        ("GET", path) if path.starts_with("/files/") => {
            let file_path = format!("{}/{}", file_directory, path.strip_prefix("/files/").unwrap());
            match fs::read_to_string(&file_path) {
                Ok(content) => {
                    let mut resp = HttpResponse::new(200)
                        .with_text_body("application/octet-stream", &content);
                    
                    if request_info.accepts_gzip {
                        resp = resp.with_gzip_compression();
                    }
                    resp
                }
                Err(_) => HttpResponse::new(404)
            }
        }
        ("POST", path) if path.starts_with("/files/") => {
            let file_path = format!("{}/{}", file_directory, path.strip_prefix("/files/").unwrap());
            match fs::write(&file_path, body) {
                Ok(_) => HttpResponse::new(201),
                Err(_) => HttpResponse::new(500)
                    .with_text_body("text/plain", "Failed to write file")
            }
        }
        _ => HttpResponse::new(404)
    };

    if request_info.close_connection {
        response = response.add_header("Connection", "close");
    }

    response.to_bytes()
}

fn gzip_bytes(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        404 => "Not Found",
        400 => "Bad Request",
        _ => "Unknown",
    }
}
