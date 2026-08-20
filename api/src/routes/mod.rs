//! Composición del router de la API.

pub mod attachments;
pub mod auth;
pub mod dashboard;
pub mod expenses;
pub mod groups;
pub mod settlements;

use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .merge(auth::routes())
        .merge(groups::routes())
        .merge(expenses::routes())
        .merge(settlements::routes())
        .merge(attachments::routes())
        .merge(dashboard::routes())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "tarascapay-api",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
