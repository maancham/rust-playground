use std::fs;
use crate::utils::RequestInfo;
use crate::response::HttpResponse;

pub async fn route_request(
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
