//! HTTP response and its constructors

use std::io;
use std::path::Path;

use tokio::fs::File;

use crate::reqres::{HttpHeader, HttpBody, StatusCode};
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
///
/// Always make sure that `dest` is urlencoded!
pub fn redirect(dest: impl Into<String>) -> HttpResponse {
    let dest = dest.into();
    HttpResponse {
        code: StatusCode::MOVED_PERMANENTLY,
        headers: vec![HttpHeader { name: "Location".to_string(), value: dest.clone() }],
        body: format!("<a href=\"{dest}\">Click here if you weren't redirected</a>\n").into(),
        content_type: "text/html; charset=utf-8".to_string(),
    }
}

// TODO Accept-Ranges: bytes
// TODO set content type for popular exts (nginx as reference)
// Don't forget about 416 Range not satisfiable!
/// Responds with a file
pub async fn file(name: &Path) -> io::Result<HttpResponse> {
    let file = File::open(name).await?;
    let metadata = file.metadata().await?;
    let len = metadata.len();

    let mut headers = vec![];
    if let Ok(time) = metadata.modified() { // fails if field not supported
        if let Some(s) = httpdate::from_systime(time) { // fails on overflow
            headers.push(HttpHeader { name: "Last-Modified".to_string(), value: s });
        }
    }

    if let Some(date) = httpdate::now() {
        headers.push(HttpHeader { name: "Date".to_string(), value: date });
    }

    Ok(HttpResponse {
        code: StatusCode::OK,
        headers,
        body: HttpBody::File { file, len },
        content_type: "".to_string(),
    })
}
