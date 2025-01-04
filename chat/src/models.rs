use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub password: String
}

#[derive(Deserialize)]
pub struct LoginInfo {
    pub username: String,
    pub password: String
}

#[derive(Deserialize)]
pub struct ClientMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub recipient: String,
    pub content: Option<String>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: Option<usize>,
    pub data: Option<Vec<u8>>
}

#[derive(Serialize)]
pub struct ErrorMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String
}

#[derive(Serialize)]
pub struct OnlineUsersResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub users: Vec<String>
}

#[derive(Serialize)]
pub struct LoginResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub token: String
}

#[derive(Serialize)]
pub struct SignupResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String
}

#[derive(Deserialize)]
pub struct HistoryRequest {
    pub token: String
}

#[derive(Serialize)]
pub struct HistoryResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub messages: Vec<String>
}
