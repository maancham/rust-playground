use std::collections::HashMap;
use httparse;

#[derive(Debug)]
pub struct RequestInfo {
    pub content_length: Option<usize>,
    pub user_agent: Option<String>,
    pub accepts_gzip: bool,
    pub close_connection: bool,
}

impl RequestInfo {
    pub fn from_headers(headers: &[httparse::Header]) -> Self {
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
