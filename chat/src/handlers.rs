use actix_web::{web, HttpResponse};
use crate::models::*;
use crate::AppState;
use actix_web::Error;
use actix_files::NamedFile;

pub async fn signup(data: web::Data<AppState>, new_user: web::Json<User>) -> HttpResponse {
    let mut users = data.users.lock().unwrap();
    if users.contains_key(&new_user.username) {
        let error = ErrorMessage {
            msg_type: "error".to_string(),
            message: "Такий користувач вже існує".to_string(),
        };
        return HttpResponse::BadRequest().json(error);
    }
    users.insert(new_user.username.clone(), new_user.into_inner());
    let response = SignupResponse {
        msg_type: "success".to_string(),
        message: "Реєстрація успішна".to_string(),
    };
    HttpResponse::Ok().json(response)
}

pub async fn login(data: web::Data<AppState>, info: web::Json<LoginInfo>) -> HttpResponse {
    let users = data.users.lock().unwrap();
    if let Some(user) = users.get(&info.username) {
        if user.password == info.password {
            let token = uuid::Uuid::new_v4().to_string();
            drop(users);
            let mut sessions = data.sessions.lock().unwrap();
            sessions.insert(token.clone(), info.username.clone());
            let response = LoginResponse {
                msg_type: "login".to_string(),
                token,
            };
            return HttpResponse::Ok().json(response);
        }
    }
    let error = ErrorMessage {
        msg_type: "error".to_string(),
        message: "Не знайдено користувача з такими обліковими даними".to_string(),
    };
    HttpResponse::Unauthorized().json(error)
}

pub async fn get_history(data: web::Data<AppState>, query: web::Query<HistoryRequest>) -> HttpResponse {
    let sessions = data.sessions.lock().unwrap();
    if let Some(username) = sessions.get(&query.token) {
        let messages = data.messages.lock().unwrap();
        if let Some(history) = messages.get(username) {
            let response = HistoryResponse {
                msg_type: "history".to_string(),
                messages: history.clone(),
            };
            return HttpResponse::Ok().json(response);
        }
        let empty_history = HistoryResponse {
            msg_type: "history".to_string(),
            messages: Vec::new(),
        };
        return HttpResponse::Ok().json(empty_history);
    }
    let error = ErrorMessage {
        msg_type: "error".to_string(),
        message: "Invalid token".to_string(),
    };
    HttpResponse::Unauthorized().json(error)
}

pub async fn get_online_users(data: web::Data<AppState>, query: web::Query<HistoryRequest>) -> HttpResponse {
    let sessions = data.sessions.lock().unwrap();
    if let Some(_username) = sessions.get(&query.token) {
        let connections = data.connections.lock().unwrap();
        let users: Vec<String> = connections.keys().cloned().collect();
        let response = OnlineUsersResponse {
            msg_type: "online_users".to_string(),
            users,
        };
        return HttpResponse::Ok().json(response);
    }
    let error = ErrorMessage {
        msg_type: "error".to_string(),
        message: "Invalid token".to_string(),
    };
    HttpResponse::Unauthorized().json(error)
}

pub async fn download_file(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HistoryRequest>,
) -> Result<NamedFile, Error> {
    let sessions = data.sessions.lock().unwrap();
    if sessions.get(&query.token).is_none() {
        return Err(actix_web::error::ErrorUnauthorized("Invalid token"));
    }

    let file_path = format!("uploads/{}", path.into_inner());
    Ok(NamedFile::open(file_path)?)
}
