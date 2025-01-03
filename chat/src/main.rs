use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use url::Url;
use uuid::Uuid;

struct ChatSession {
    username: String,
    app_state: web::Data<AppState>,
}

impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        let mut connections = self.app_state.connections.lock().unwrap();
        connections.insert(self.username.clone(), addr);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        let mut connections = self.app_state.connections.lock().unwrap();
        connections.remove(&self.username);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct BroadcastMessage(String);

impl Handler<BroadcastMessage> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: BroadcastMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                let parsed: serde_json::Result<ClientMessage> = serde_json::from_str(&text);
                match parsed {
                    Ok(client_msg) => {
                        if client_msg.recipient == "public" {
                            // Public message
                            let message = format!("{}: {}", self.username, client_msg.content);

                            // Store message for all users
                            let mut messages = self.app_state.messages.lock().unwrap();
                            for user in self.app_state.users.lock().unwrap().keys() {
                                let user_history = messages.entry(user.clone()).or_insert(Vec::new());
                                user_history.push(message.clone());
                            }

                            // Broadcast to all connected clients
                            let connections = self.app_state.connections.lock().unwrap();
                            for (user, addr) in connections.iter() {
                                addr.do_send(BroadcastMessage(message.clone()));
                            }
                        } else {
                            // Private message
                            let to = client_msg.recipient.clone();
                            let content = client_msg.content.clone();
                            let connections = self.app_state.connections.lock().unwrap();
                            if let Some(addr) = connections.get(&to) {
                                addr.do_send(PrivateMessage { from: self.username.clone(), to: to.clone(), content: content.clone() });

                                // Store message history
                                let mut messages = self.app_state.messages.lock().unwrap();
                                let sender_history = messages.entry(self.username.clone()).or_insert(Vec::new());
                                sender_history.push(format!("To {}: {}", to, content));
                                let recipient_history = messages.entry(to.clone()).or_insert(Vec::new());
                                recipient_history.push(format!("From {}: {}", self.username, content));
                            } else {
                                ctx.text("User not found");
                            }
                        }
                    },
                    Err(_) => ctx.text("Invalid message format"),
                }
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

#[derive(Deserialize)]
struct ClientMessage {
    recipient: String,
    content: String,
}

#[derive(Message)]
#[rtype(result = "()")]
struct PrivateMessage {
    from: String,
    to: String,
    content: String,
}

impl Handler<PrivateMessage> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: PrivateMessage, ctx: &mut Self::Context) {
        let formatted = format!("Private from {}: {}", msg.from, msg.content);
        ctx.text(formatted);
    }
}

#[derive(Deserialize)]
struct HistoryRequest {
    token: String,
}

async fn get_history(data: web::Data<AppState>, query: web::Query<HistoryRequest>) -> HttpResponse {
    let sessions = data.sessions.lock().unwrap();
    if let Some(username) = sessions.get(&query.token) {
        let messages = data.messages.lock().unwrap();
        if let Some(history) = messages.get(username) {
            return HttpResponse::Ok().json(history);
        }
        return HttpResponse::Ok().json(Vec::<String>::new());
    }
    HttpResponse::Unauthorized().body("Invalid token")
}


#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    username: String,
    password: String
}

struct AppState {
    users: Mutex<HashMap<String, User>>,
    sessions: Mutex<HashMap<String, String>>, // token -> username
    connections: Mutex<HashMap<String, Addr<ChatSession>>>, // username -> WebSocket address
    messages: Mutex<HashMap<String, Vec<String>>>, // username -> message history
}

impl AppState {
    fn new() -> Self {
        AppState {
            users: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            connections: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new())
        }
    }
}

#[derive(Deserialize)]
struct LoginInfo {
    username: String,
    password: String,
}

async fn signup(data: web::Data<AppState>, new_user: web::Json<User>) -> HttpResponse {
    let mut users = data.users.lock().unwrap();
    if users.contains_key(&new_user.username) {
        return HttpResponse::BadRequest().body("Username already exists");
    }
    users.insert(new_user.username.clone(), new_user.into_inner());
    HttpResponse::Ok().body("User registered successfully")
}

async fn login(data: web::Data<AppState>, info: web::Json<LoginInfo>) -> HttpResponse {
    let users = data.users.lock().unwrap();
    if let Some(user) = users.get(&info.username) {
        if user.password == info.password {
            let token = Uuid::new_v4().to_string();
            drop(users);
            let mut sessions = data.sessions.lock().unwrap();
            sessions.insert(token.clone(), info.username.clone());
            return HttpResponse::Ok().json(serde_json::json!({ "token": token }));
        }
    }
    HttpResponse::Unauthorized().body("Invalid credentials")
}

async fn ws_handler(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let query = req.query_string();
    let url = Url::parse(&format!("http://localhost/?{}", query)).unwrap();
    let token = url.query_pairs().find(|(k, _)| k == "token").map(|(_, v)| v.to_string());

    if let Some(token) = token {
        let sessions = data.sessions.lock().unwrap();
        if let Some(username) = sessions.get(&token) {
            // Valid token, establish WebSocket connection
            return ws::start(ChatSession { username: username.clone(), app_state: data.clone() }, &req, stream);
        }
    }
    // Invalid token
    Ok(HttpResponse::Unauthorized().body("Unauthorized"))
}

#[derive(Serialize)]
struct OnlineUsersResponse {
    users: Vec<String>,
}

async fn get_online_users(data: web::Data<AppState>, query: web::Query<HistoryRequest>) -> HttpResponse {
    let sessions = data.sessions.lock().unwrap();
    if let Some(username) = sessions.get(&query.token) {
        let connections = data.connections.lock().unwrap();
        let users: Vec<String> = connections.keys().cloned().collect();
        return HttpResponse::Ok().json(OnlineUsersResponse { users });
    }
    HttpResponse::Unauthorized().body("Invalid token")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState::new());

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/ws/", web::get().to(ws_handler))
            .route("/signup", web::post().to(signup))
            .route("/login", web::post().to(login))
            .route("/history", web::get().to(get_history))
            .route("/online_users", web::get().to(get_online_users))
            .service(fs::Files::new("/", ".").index_file("index.html"))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
