use actix::prelude::*;
use actix_web::web;
use actix_web_actors::ws;
use serde_json::json;
use crate::models::*;
use crate::AppState;

#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastMessage(pub String);

#[derive(Message)]
#[rtype(result = "()")]
pub struct PrivateMessage {
    pub from: String,
    pub content: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UserConnected {
    pub username: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UserDisconnected {
    pub username: String,
}

pub struct ChatSession {
    pub username: String,
    pub app_state: web::Data<AppState>,
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

impl Handler<BroadcastMessage> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: BroadcastMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl Handler<PrivateMessage> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: PrivateMessage, ctx: &mut Self::Context) {
        let formatted = format!("Приватне повідомлення від {}: {}", msg.from, msg.content);
        ctx.text(formatted);
    }
}

impl Handler<UserConnected> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: UserConnected, ctx: &mut Self::Context) {
        let notification = json!({
            "type": "user_connected",
            "username": msg.username
        }).to_string();
        ctx.text(notification);
    }
}

impl Handler<UserDisconnected> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: UserDisconnected, ctx: &mut Self::Context) {
        let notification = json!({
            "type": "user_disconnected",
            "username": msg.username
        }).to_string();
        ctx.text(notification);
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
                            let error = ErrorMessage {
                                msg_type: "error".to_string(),
                                message: "Невідомий тип повідомлення".to_string(),
                            };
                            ctx.text(json!(error).to_string());
                        }
                    },
                    Err(_) => {
                        let error = ErrorMessage {
                            msg_type: "error".to_string(),
                            message: "Невідомий формат повідомлення".to_string(),
                        };
                        ctx.text(json!(error).to_string());
                    },
                }
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

impl ChatSession {
    pub fn handle_text_message(&mut self, client_msg: ClientMessage, ctx: &mut ws::WebsocketContext<Self>) {
        let recipient = client_msg.recipient.clone();
        let content = client_msg.content.clone().unwrap_or_default();

        if recipient == "public" {
            let message = json!({
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
            if recipient == self.username {
                let error = ErrorMessage {
                    msg_type: "error".to_string(),
                    message: "Cannot send messages to yourself.".to_string(),
                };
                ctx.text(json!(error).to_string());
                return;
            }

            let to = recipient.clone();
            let connections = self.app_state.connections.lock().unwrap();

            if let Some(addr) = connections.get(&to) {
                let private_msg = json!({
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
                let error = ErrorMessage {
                    msg_type: "error".to_string(),
                    message: "Користувач не знайдений".to_string(),
                };
                ctx.text(json!(error).to_string());
            }
        }
    }

    pub fn handle_file_message(&mut self, client_msg: ClientMessage, ctx: &mut ws::WebsocketContext<Self>) {
        if let Some(data) = client_msg.data {
            let recipient = client_msg.recipient.clone();
            let filename = client_msg.filename.clone().unwrap_or_default();

            let file_id = uuid::Uuid::new_v4().to_string();
            let file_path = format!("uploads/{}", file_id);
            std::fs::create_dir_all("uploads").unwrap();
            std::fs::write(&file_path, data).unwrap();

            let metadata_message = json!({
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
                if recipient == self.username {
                    let error = ErrorMessage {
                        msg_type: "error".to_string(),
                        message: "Cannot send files to yourself.".to_string(),
                    };
                    ctx.text(json!(error).to_string());
                    return;
                }

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
                    let error = ErrorMessage {
                        msg_type: "error".to_string(),
                        message: "Користувач не знайдений".to_string(),
                    };
                    ctx.text(json!(error).to_string());
                }
            }
        } else {
            let error = ErrorMessage {
                msg_type: "error".to_string(),
                message: "Не вибрано жодного файлу".to_string(),
            };
            ctx.text(json!(error).to_string());
        }
    }
}
