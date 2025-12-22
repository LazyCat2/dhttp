use std::io;

use dhttp::prelude::*;
use dhttp::reqres::res;

struct MyService {
    name: String,
}

impl HttpService for MyService {
    async fn request(&self, _route: &str, req: &HttpRequest, _body: &mut dyn HttpRead) -> HttpResult {
        // TODO: could parse the user agent here
        let user = req.get_header("User-Agent").unwrap_or_default();
        let name = &self.name;

        let greeting = format!("Hello, {user}! My name is {name}. Have a nice day\n");
        Ok(res::text(greeting))
    }
}

fn main() -> io::Result<()> {
    dhttp::tokio_rt()?.block_on(http_main())
}

async fn http_main() -> io::Result<()> {
    let mut server = HttpServer::new();
    let name = "drakohttp".to_string();
    server.service(MyService { name });

    dhttp::serve_tcp("[::]:8080", server).await
}
