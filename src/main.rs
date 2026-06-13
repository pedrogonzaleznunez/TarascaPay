use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};


#[derive(Deserialize)]
struct UserRequest {
    username: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: u64,
    username: String,
    status: String,
}

#[tokio::main]
async fn main() {

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/users", post(create_user));


    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("Running on http://127.0.0.1:3000");
    
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn create_user(Json(payload): Json<UserRequest>) -> Json<UserResponse> {
    let response = UserResponse {
        id: 101,
        username: payload.username,
        status: "active".to_string(),
    };

    Json(response)
}