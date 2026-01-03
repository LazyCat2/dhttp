use std::io;
use std::time::Duration;
use dhttp::prelude::*;
use dhttp::reqres::res;
use dhttp::reqres::sse::{HttpSse, SseEvent};

struct SseService;
struct SseHandler {
    counter: u8
}


impl HttpService for SseService {
    async fn request(&self, _route: &str, req: &HttpRequest, _body: &mut dyn HttpRead) -> HttpResult {
        Ok(res::sse(SseHandler { counter: 0 }))
    }
}

impl HttpSse for SseHandler {
    async fn next(&mut self) -> Option<SseEvent> {
        self.counter += 1;

        tokio::time::sleep(Duration::from_millis(1000)).await;

        match self.counter {
            1 => Some(
                SseEvent {
                    // Non-standard event names are supported
                    name: "warning".to_string(),
                    data: "Dragons ahead".to_string(),
                }
            ),

            // Server closes connection once this function
            // returns None
            10 => None,

            _ => Some(
                SseEvent {
                    name: "message".to_string(),
                    data: format!("{}", self.counter)
                }
            )
        }
    }
}

fn main() -> io::Result<()> {
    dhttp::tokio_rt()?.block_on(http_main())
}

async fn http_main() -> io::Result<()> {
    let mut server = HttpServer::new();
    server.service(SseService);

    dhttp::serve_tcp("[::]:8080", server).await
}
