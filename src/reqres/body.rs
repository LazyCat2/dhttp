use std::io;
use std::fmt;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::core::connection::HttpWrite;
use crate::util::escape;

/// Body of the response
pub enum HttpBody {
    /// In-memory bytes
    Bytes(Vec<u8>),
    /// File handle to read
    File { file: File, len: u64 },
    // Stream(Box<dyn HttpStream>)
    // Upgrade(Box<dyn HttpUpgrade>)
}

impl HttpBody {
    /// Length of body in bytes
    pub fn content_length(&self) -> u64 {
        match self {
            HttpBody::Bytes(v) => v.len() as u64,
            HttpBody::File { len, .. } => *len,
        }
    }

    /// Send this body into writer
    pub async fn send(&mut self, conn: &mut dyn HttpWrite) -> io::Result<()> {
        match self {
            HttpBody::Bytes(body) => { conn.write_all(body).await?; }
            HttpBody::File { file, .. } => { tokio::io::copy(file, conn).await?; }
        }
        Ok(())
    }
}

impl fmt::Debug for HttpBody {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpBody::Bytes(v) => write!(fmt, r#"HttpBody::Bytes(b"{}")"#, escape::to_utf8(v)),
            // sigh. debug formatter for file struct sucks so much for both std and tokio
            HttpBody::File { file, len } => fmt.debug_struct("HttpBody::File").field("file", file).field("len", len).finish(),
        }
    }
}

impl From<Vec<u8>> for HttpBody {
    fn from(v: Vec<u8>) -> HttpBody {
        HttpBody::Bytes(v)
    }
}

impl From<String> for HttpBody {
    fn from(s: String) -> HttpBody {
        HttpBody::Bytes(s.into_bytes())
    }
}

impl From<&str> for HttpBody {
    fn from(s: &str) -> HttpBody {
        HttpBody::Bytes(s.to_string().into_bytes())
    }
}
