use std::io::Write;
use flate2::Compression;
use flate2::write::GzEncoder;

pub struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn with_text_body(mut self, content_type: &str, body: &str) -> Self {
        self.headers.push(("Content-Type".to_string(), content_type.to_string()));
        self.body = body.as_bytes().to_vec();
        self
    }

    pub fn with_gzip_compression(mut self) -> Self {
        if !self.body.is_empty() {
            self.body = gzip_bytes(&self.body);
            self.headers.push(("Content-Encoding".to_string(), "gzip".to_string()));
        }
        self
    }

    pub fn add_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn to_bytes(self) -> Vec<u8> {
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
        500 => "Internal Server Error",
        _ => "Unknown",
    }
} 
