use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use actix_files::NamedFile;
use actix_web::error::ErrorUnauthorized;
use url::Url;
use uuid::Uuid;

struct ChatSession {
    username: String,
    app_state: web::Data<AppState>,
}

impl ChatSession {
    fn handle_text_message(&mut self, client_msg: ClientMessage, ctx: &mut ws::WebsocketContext<Self>) {
        let recipient = client_msg.recipient.clone();
        let content = client_msg.content.clone().unwrap_or_default();

        if recipient == "public" {
            let message = serde_json::json!({
                "type": "public",
                "from": self.username,
                "content": content
            }).to_string();

            {
                let mut messages = self.app_state.messages.lock().unwrap();
                for user in self.app_state.users.lock().unwrap().keys() {
                    let user_history = messages.entry(user.clone()).or_insert(Vec::new());
                    user_history.push(format!("{}: {}", self.username, content));
                }
            }

            let connections = self.app_state.connections.lock().unwrap();
            for (_user, addr) in connections.iter() {
                addr.do_send(BroadcastMessage(message.clone()));
            }
        } else {
            let to = recipient.clone();
            let connections = self.app_state.connections.lock().unwrap();

            if let Some(addr) = connections.get(&to) {
                let private_msg = serde_json::json!({
                    "type": "private",
                    "from": self.username,
                    "content": content
                }).to_string();

                addr.do_send(PrivateMessage { from: self.username.clone(), content: private_msg.clone() });

                ctx.text(private_msg.clone());

                {
                    let mut messages = self.app_state.messages.lock().unwrap();
                    let sender_history = messages.entry(self.username.clone()).or_insert(Vec::new());
                    sender_history.push(format!("До {}: {}", to, content));
                    let recipient_history = messages.entry(to.clone()).or_insert(Vec::new());
                    recipient_history.push(format!("Від {}: {}", self.username, content));
                }
            } else {
                let error = serde_json::json!({
                    "type": "error",
                    "message": "Користувач не знайдений"
                }).to_string();
                ctx.text(error);
            }
        }
    }

    fn handle_file_message(&mut self, client_msg: ClientMessage, ctx: &mut ws::WebsocketContext<Self>) {
        if let Some(data) = client_msg.data {
            let recipient = client_msg.recipient.clone();
            let filename = client_msg.filename.clone().unwrap_or_default();

            let file_id = Uuid::new_v4().to_string();
            let file_path = format!("uploads/{}", file_id);
            std::fs::create_dir_all("uploads").unwrap();
            std::fs::write(&file_path, data).unwrap();

            let metadata_message = serde_json::json!({
                "type": "file",
                "from": self.username,
                "fileId": file_id,
                "filename": filename
            }).to_string();

            if recipient == "public" {
                let connections = self.app_state.connections.lock().unwrap();
                for (_user, addr) in connections.iter() {
                    addr.do_send(BroadcastMessage(metadata_message.clone()));
                }
            } else {
                let connections = self.app_state.connections.lock().unwrap();
                if let Some(addr) = connections.get(&recipient) {
                    addr.do_send(PrivateMessage { from: self.username.clone(), content: metadata_message.clone() });

                    ctx.text(metadata_message.clone());

                    {
                        let mut messages = self.app_state.messages.lock().unwrap();
                        let sender_history = messages.entry(self.username.clone()).or_insert(Vec::new());
                        sender_history.push(format!("До {}: Надіслав файл '{}'", recipient, filename));
                        let recipient_history = messages.entry(recipient.clone()).or_insert(Vec::new());
                        recipient_history.push(format!("Від {}: Отримано файл '{}'", self.username, filename));
                    }
                } else {
                    let error = serde_json::json!({
                        "type": "error",
                        "message": "Користувач не знайдений"
                    }).to_string();
                    ctx.text(error);
                }
            }
        } else {
            let error = serde_json::json!({
                "type": "error",
                "message": "Не вибрано жодного файлу"
            }).to_string();
            ctx.text(error);
        }
    }
}

impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        let username = self.username.clone();
        {
            let mut connections = self.app_state.connections.lock().unwrap();
            connections.insert(username.clone(), addr);
        }

        let connections = self.app_state.connections.lock().unwrap();
        for (user, addr) in connections.iter() {
            if user != &username {
                addr.do_send(UserConnected {
                    username: username.clone(),
                });
            }
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        let username = self.username.clone();
        {
            let mut connections = self.app_state.connections.lock().unwrap();
            connections.remove(&username);
        }

        let connections = self.app_state.connections.lock().unwrap();
        for (_user, addr) in connections.iter() {
            addr.do_send(UserDisconnected {
                username: username.clone(),
            });
        }
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
                        if client_msg.msg_type == "file" {
                            self.handle_file_message(client_msg, ctx);
                        } else if client_msg.msg_type == "message" {
                            self.handle_text_message(client_msg, ctx);
                        } else {
                            let error = serde_json::json!({
                                "type": "error",
                                "message": "Невідомий тип повідомлення"
                            }).to_string();

                            ctx.text(error);
                        }
                    },
                    Err(_) => {
                        let error = serde_json::json!({
                            "type": "error",
                            "message": "Невідомий формат повідомлення"
                        }).to_string();

                        ctx.text(error);
                    },
                }
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

#[derive(Serialize)]
struct ErrorMessage {
    r#type: String,
    message: String,
}

#[derive(Deserialize)]
struct ClientMessage {
    #[serde(rename = "type")]
    msg_type: String,
    recipient: String,
    content: Option<String>,
    filename: Option<String>,
    content_type: Option<String>,
    size: Option<usize>,
    data: Option<Vec<u8>>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct PrivateMessage {
    from: String,
    content: String,
}

impl Handler<PrivateMessage> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: PrivateMessage, ctx: &mut Self::Context) {
        let formatted = format!("Приватне повідомлення від {}: {}", msg.from, msg.content);
        ctx.text(formatted);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct UserConnected {
    username: String,
}

#[derive(Message)]
#[rtype(result = "()")]
struct UserDisconnected {
    username: String,
}

impl Handler<UserConnected> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: UserConnected, ctx: &mut Self::Context) {
        let notification = serde_json::json!({
            "type": "user_connected",
            "username": msg.username
        }).to_string();
        ctx.text(notification);
    }
}

impl Handler<UserDisconnected> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: UserDisconnected, ctx: &mut Self::Context) {
        let notification = serde_json::json!({
            "type": "user_disconnected",
            "username": msg.username
        }).to_string();
        ctx.text(notification);
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
    sessions: Mutex<HashMap<String, String>>,
    connections: Mutex<HashMap<String, Addr<ChatSession>>>,
    messages: Mutex<HashMap<String, Vec<String>>>
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
        return HttpResponse::BadRequest().body("Такий користувач вже існує");
    }
    users.insert(new_user.username.clone(), new_user.into_inner());
    HttpResponse::Ok().body("Реєстрація успішна")
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
    HttpResponse::Unauthorized().body("Не знайдено користувача з такими обліковими даними")
}

async fn ws_handler(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let query = req.query_string();
    let url = Url::parse(&format!("http://localhost/?{}", query)).unwrap();
    let token = url.query_pairs().find(|(k, _)| k == "token").map(|(_, v)| v.to_string());

    if let Some(token) = token {
        let sessions = data.sessions.lock().unwrap();
        if let Some(username) = sessions.get(&token) {
            return ws::start(ChatSession { username: username.clone(), app_state: data.clone() }, &req, stream);
        }
    }

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

async fn download_file(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HistoryRequest>,
) -> Result<NamedFile, Error> {
    let sessions = data.sessions.lock().unwrap();
    if sessions.get(&query.token).is_none() {
        return Err(ErrorUnauthorized(format!("No session found for token {}", query.token)));
    }

    let file_path = format!("uploads/{}", path.into_inner());
    Ok(NamedFile::open(file_path)?)
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
            .route("/download/{file_id}", web::get().to(download_file))
            .service(fs::Files::new("/", ".").index_file("index.html"))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
