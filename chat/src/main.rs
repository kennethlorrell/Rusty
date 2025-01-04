mod models;
mod handlers;
mod websocket;

use actix_files as fs;
use actix_web::{web, App, HttpServer};
use models::*;
use handlers::*;
use websocket::*;
use std::collections::HashMap;
use std::sync::Mutex;
use actix::Addr;
use actix_web_actors::ws;
use url::Url;

pub struct AppState {
    pub users: Mutex<HashMap<String, User>>,
    pub sessions: Mutex<HashMap<String, String>>,
    pub connections: Mutex<HashMap<String, Addr<ChatSession>>>,
    pub messages: Mutex<HashMap<String, Vec<String>>>
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        users: Mutex::new(HashMap::new()),
        sessions: Mutex::new(HashMap::new()),
        connections: Mutex::new(HashMap::new()),
        messages: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/ws/", web::get().to(websocket_handler))
            .route("/signup", web::post().to(signup))
            .route("/login", web::post().to(login))
            .route("/history", web::get().to(get_history))
            .route("/online_users", web::get().to(get_online_users))
            .route("/download/{file_id}", web::get().to(download_file))
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

async fn websocket_handler(req: actix_web::HttpRequest, stream: web::Payload, data: web::Data<AppState>) -> Result<actix_web::HttpResponse, actix_web::Error> {
    use websocket::ChatSession;
    use actix_web::Error;

    let query = req.query_string();
    let url = Url::parse(&format!("http://localhost/?{}", query)).map_err(|_| Error::from(actix_web::error::ErrorBadRequest("Invalid URL")))?;
    let token = url.query_pairs().find(|(k, _)| k == "token").map(|(_, v)| v.to_string());

    if let Some(token) = token {
        let sessions = data.sessions.lock().unwrap();
        if let Some(username) = sessions.get(&token) {
            let chat_session = ChatSession {
                username: username.clone(),
                app_state: data.clone(),
            };
            return ws::start(chat_session, &req, stream);
        }
    }

    Ok(actix_web::HttpResponse::Unauthorized().body("Unauthorized"))
}
