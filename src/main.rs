use std::fs;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use clap::Parser;

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
            handle_connection(stream, file_directory).await
        });
    }
}

async fn handle_connection(mut stream: TcpStream, file_directory: String) {
    let mut buf = [0; 1024];
    let read_bufsize = stream.read(&mut buf).await.unwrap();

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    let request_result = req.parse(&buf[0..read_bufsize]);

    match request_result {
        Ok(httparse::Status::Complete(headers_end)) => {
            let method = req.method.unwrap();
            let path = req.path.unwrap();

            let content_length = find_header(&req.headers, "content-length");
            let user_agent = find_header(&req.headers, "user-agent");
            let accepted_encodings = find_header(&req.headers, "accept-encoding");

            let mut content_encoding = None;
            if let Some(encodings) = accepted_encodings {
                let gzip_present = encodings.split(",").map(|e| e.trim()).any(|e| e == "gzip");
                if gzip_present {
                    content_encoding = Some("gzip");
                }
            }

            let response = match (method, path) {
                ("GET", "/") => &format_response(200, "", "", content_encoding),
                ("GET", "/user-agent") => {
                    if let Some(agent) = user_agent {
                        &format_response(200, "text/plain", agent, content_encoding)
                    }
                    else {
                        &format_response(404, "", "", content_encoding)
                    }
                }
                ("GET", path) if path.starts_with("/echo/") => {
                    if let Some(post_path) = path.strip_prefix("/echo/") {
                        &format_response(200, "text/plain", post_path, content_encoding)
                    } else {
                        &format_response(404, "", "", content_encoding)
                    }
                }
                ("GET", path) if path.starts_with("/files/") => {
                    let file_path = format!("{}/{}", file_directory, path.strip_prefix("/files/").unwrap());
                    if let Ok(file_content) = fs::read_to_string(file_path) {
                        &format_response(200, "application/octet-stream", &file_content, content_encoding)
                    } else {
                        &format_response(404, "", "", content_encoding)
                    }
                }
                ("POST", path) if path.starts_with("/files/") => {
                    let file_path = format!("{}/{}", file_directory, path.strip_prefix("/files/").unwrap());

                    let mut body = Vec::new();
                    body.extend_from_slice(&buf[headers_end..read_bufsize]);

                    if let Some(content_len_str) = content_length {
                        if let Ok(content_len) = content_len_str.parse::<usize>() {
                            let remaining_bytes = content_len.saturating_sub(body.len());
                            if remaining_bytes > 0 {
                                let mut remaining_buf = vec![0; remaining_bytes];
                                stream.read_exact(&mut remaining_buf).await.unwrap();
                                body.extend_from_slice(&remaining_buf);
                            }
                        }
                    }

                    fs::write(file_path, body).unwrap();
                    &format_response(201, "", "", content_encoding)
                }
                _ =>  &format_response(404, "", "", content_encoding),
            };

            stream
                .write_all(response.as_bytes())
                .await.unwrap();

        }
        Ok(httparse::Status::Partial) => {
            println!("received partial request");
        },
        Err(e) => {
            println!("failed to parse request: {:?}", e);
        }
    }
}

fn format_response(status: u16, content_type: &str, body: &str, content_encoding: Option<&str>) -> String {
    let mut response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n",
        status,
        status_text(status),
        content_type,
        body.len(),
    );

    if let Some(encoding) = content_encoding {
        response += &format!("Content-Encoding: {}\r\n", encoding);
    }

    response += &format!("\r\n{}", body);

    response
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

fn find_header<'a>(headers: &'a [httparse::Header], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|head| head.name.to_lowercase() == name.to_lowercase())
        .and_then(|head| std::str::from_utf8(head.value).ok())
}
