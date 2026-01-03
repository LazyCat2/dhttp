use std::fmt::{Display, Formatter};
use std::io;
use tokio::io::AsyncWriteExt;
use crate::core::connection::HttpConnection;
use crate::reqres::{HttpResponse, HttpUpgrade};
use crate::reqres::body::HttpUpgradeRaw;

pub struct SseEvent {
    pub name: String,
    pub data: String,
}

impl Display for SseEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f, "event: {}\ndata: {}\n\n",
            self.name, self.data
        )
    }
}


/// Streaming SSE over HTTP.
/// To be used in [`reqres::res::sse`].
#[doc(alias = "SSE", alias = "EventSource")]
pub trait HttpSse: Send + 'static {
    fn next(&mut self) -> impl Future<Output = Option<SseEvent>> + Send;
}

impl<T: HttpSse> HttpUpgrade for T {
    async fn upgrade(&mut self, conn: &mut dyn HttpConnection) -> io::Result<()> {
        while let Some(event) = self.next().await {
            conn.write_all(event.to_string().as_bytes()).await?;
        }

        Ok(())
    }
}
