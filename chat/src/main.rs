use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

// WebSocket session actor
struct ChatSession;

impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self>;
}

// Handle incoming WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                // Broadcast the message to all connected clients
                ctx.text(text);
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

// WebSocket route handler
async fn ws_handler(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(ChatSession {}, &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Start HTTP server
    HttpServer::new(|| {
        App::new()
            .route("/ws/", web::get().to(ws_handler))
            .service(fs::Files::new("/", ".").index_file("index.html"))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
