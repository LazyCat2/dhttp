use std::io;

use dhttp::prelude::*;
use dhttp::reqres::res;
use tokio::sync::Mutex;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

struct MyService {
    agents: Mutex<File>,
}

impl HttpService for MyService {
    async fn request(&self, _route: &str, req: &HttpRequest, _body: &mut dyn HttpRead) -> HttpResult {
        let mut agent = req.get_header("User-Agent").unwrap_or_default().to_string();
        agent.push('\n');
        self.agents.lock().await.write_all(agent.as_bytes()).await?;
        self.agents.lock().await.flush().await?;

        Ok(res::html(agent))
    }
}

fn main() -> io::Result<()> {
    dhttp::tokio_rt()?.block_on(http_main())
}

async fn http_main() -> io::Result<()> {
    let mut server = HttpServer::new();
    let agents = Mutex::new(OpenOptions::new().write(true).append(true).create(true).open("myagents.txt").await?);
    server.service(MyService { agents });

    dhttp::serve_tcp("[::]:8080", server).await
}
