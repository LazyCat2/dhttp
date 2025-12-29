//! HTTP response and its constructors

use std::io::SeekFrom;
use std::path::Path;

use tokio::io::AsyncSeekExt;
use tokio::fs::File;

use percent_encoding_lite::{is_encoded, encode, Bitmask};

use crate::core::HttpResult;
use crate::reqres::{HttpRequest, HttpHeader, HttpBody, StatusCode};
use crate::util::httpdate;

/// Your response
#[derive(Debug)]
pub struct HttpResponse {
    pub code: StatusCode,
    pub headers: Vec<HttpHeader>,
    pub body: HttpBody,
    pub content_type: String,
}

impl HttpResponse {
    /// An empty response
    pub fn new() -> HttpResponse {
        HttpResponse::with_type("", vec![])
    }

    /// Pushes a new header
    pub fn add_header(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut HttpResponse {
        self.headers.push(HttpHeader { name: name.into(), value: value.into() });
        self
    }

    /// Constructs new response with a specified `Content-Type`
    pub fn with_type(content_type: impl Into<String>, body: impl Into<HttpBody>) -> HttpResponse {
        HttpResponse {
            code: StatusCode::OK,
            headers: vec![],
            body: body.into(),
            content_type: content_type.into(),
        }
    }
}

impl Default for HttpResponse {
    fn default() -> HttpResponse {
        HttpResponse::new()
    }
}

/// Response of bytes (`application/octet-stream`)
pub fn bytes(bytes: Vec<u8>) -> HttpResponse {
    HttpResponse::with_type("application/octet-stream", bytes)
}

/// Plaintext response (`text/plain`)
pub fn text(text: impl Into<String>) -> HttpResponse {
    HttpResponse::with_type("text/plain; charset=utf-8", text.into())
}

/// HTML response (`text/html`)
pub fn html(html: impl Into<String>) -> HttpResponse {
    HttpResponse::with_type("text/html; charset=utf-8", html.into())
}

/// JSON response (`application/json`)
pub fn json(json: impl Into<String>) -> HttpResponse {
    HttpResponse::with_type("application/json", json.into())
}

/// HTTP redirect with the `Location` header
pub fn redirect(dest: impl Into<String>) -> HttpResponse {
    let mut dest = dest.into();
    // To avoid XSS for URLs containing a double quote or back slash
    if !is_encoded(&dest, Bitmask::URI) {
        dest = encode(dest, Bitmask::URI);
    }
    HttpResponse {
        code: StatusCode::MOVED_PERMANENTLY,
        headers: vec![HttpHeader { name: "Location".to_string(), value: dest.clone() }],
        body: format!("<a href=\"{dest}\">Click here if you weren't redirected</a>\n").into(),
        content_type: "text/html; charset=utf-8".to_string(),
    }
}

// TODO set content type for popular exts (nginx as reference)
/// Responds with a file
pub async fn file(req: &HttpRequest, name: &Path) -> HttpResult {
    let mut file = File::open(name).await?;
    let metadata = file.metadata().await?;
    let mut len = metadata.len();

    // becomes PARTIAL_CONTENT if range was served
    let mut code = StatusCode::OK;

    // Last-Modified
    let mut headers = vec![];
    if let Ok(time) = metadata.modified() { // fails if field not supported
        if let Some(s) = httpdate::from_systime(time) { // fails on overflow
            headers.push(HttpHeader { name: "Last-Modified".to_string(), value: s });
        }
    }

    // Date
    if let Some(date) = httpdate::now() {
        headers.push(HttpHeader { name: "Date".to_string(), value: date });
    }

    // Advertise byte ranges support
    headers.push(HttpHeader {
        name: "Accept-Ranges".to_string(),
        value: "bytes".to_string(),
    });

    // Parse byte range request
    if let Some(range) = req.get_header("Range") {
        if let Some((start, mut end)) = parse_range(range) && start <= len && start <= end {
            end = end.min(len);

            headers.push(HttpHeader {
                name: "Content-Range".to_string(),
                value: format!("bytes {start}-{end}/{len}"),
            });

            file.seek(SeekFrom::Start(start)).await?;
            len = end - start + 1;
            code = StatusCode::PARTIAL_CONTENT;
        } else {
            // we have to set Content-Range in case of error too but errors can't have headers in dhttp
            return Err(StatusCode::RANGE_NOT_SATISFIABLE.into());
        }
    }

    Ok(HttpResponse {
        code,
        headers,
        body: HttpBody::File { file, len },
        content_type: "".to_string(),
    })
}

fn parse_range(range: &str) -> Option<(u64, u64)> {
    let (start, end) = range.strip_prefix("bytes=")?.split_once('-')?;
    let start = if start.is_empty() { 0 } else { start.parse().ok()? };
    let end = if end.is_empty() { u64::MAX } else { end.parse().ok()? };
    Some((start, end))
}
