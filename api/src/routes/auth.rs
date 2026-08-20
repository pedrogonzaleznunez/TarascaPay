//! Registro, inicio de sesión y perfil.

use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use shared::{
    AuthResponse, ChangePasswordRequest, LoginRequest, MessageResponse, RegisterRequest,
    UpdateProfileRequest, UserResponse,
};

use crate::auth::{hash_password, issue_token, verify_password, AuthUser};
use crate::error::{AppError, AppResult};
use crate::models::UserRow;
use crate::repo;
use crate::state::AppState;
use crate::validate;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/profile", put(update_profile))
        .route("/auth/password", put(change_password))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    let email = validate::email(&payload.email)?;
    validate::password(&payload.password)?;
    let display_name = validate::non_empty(&payload.display_name, "el nombre", 80)?;
    let currency = validate::currency(payload.currency.as_deref().unwrap_or("ARS"))?;

    if repo::find_user_by_email(&state.db, &email).await?.is_some() {
        return Err(AppError::conflict("ya existe una cuenta con ese email"));
    }

    let password_hash = hash_password(&payload.password)?;

    let user = sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (email, password_hash, display_name, currency)
         VALUES ($1, $2, $3, $4)
         RETURNING id, email, password_hash, display_name, avatar_url, currency, created_at",
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(&display_name)
    .bind(&currency)
    .fetch_one(&state.db)
    .await?;

    let token = issue_token(user.id, &state.config.jwt_secret, state.config.jwt_ttl_hours)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into_response(),
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let email = payload.email.trim().to_lowercase();

    let user = repo::find_user_by_email(&state.db, &email).await?;

    // Mismo mensaje para email inexistente y contraseña incorrecta: no
    // revelamos qué emails están registrados.
    let user = match user {
        Some(u) if verify_password(&payload.password, &u.password_hash) => u,
        _ => return Err(AppError::unauthorized("email o contraseña incorrectos")),
    };

    let token = issue_token(user.id, &state.config.jwt_secret, state.config.jwt_ttl_hours)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into_response(),
    }))
}

async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<UserResponse>> {
    let row = repo::find_user_by_id(&state.db, user.id).await?;
    Ok(Json(row.into_response()))
}

async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<UpdateProfileRequest>,
) -> AppResult<Json<UserResponse>> {
    let display_name = payload
        .display_name
        .as_deref()
        .map(|n| validate::non_empty(n, "el nombre", 80))
        .transpose()?;
    let currency = payload
        .currency
        .as_deref()
        .map(validate::currency)
        .transpose()?;

    let row = sqlx::query_as::<_, UserRow>(
        "UPDATE users SET
            display_name = COALESCE($2, display_name),
            avatar_url   = COALESCE($3, avatar_url),
            currency     = COALESCE($4, currency),
            updated_at   = NOW()
         WHERE id = $1
         RETURNING id, email, password_hash, display_name, avatar_url, currency, created_at",
    )
    .bind(user.id)
    .bind(display_name)
    .bind(payload.avatar_url)
    .bind(currency)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row.into_response()))
}

async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<MessageResponse>> {
    validate::password(&payload.new_password)?;

    let row = repo::find_user_by_id(&state.db, user.id).await?;
    if !verify_password(&payload.current_password, &row.password_hash) {
        return Err(AppError::unauthorized("la contraseña actual no es correcta"));
    }

    let new_hash = hash_password(&payload.new_password)?;
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = NOW() WHERE id = $1")
        .bind(user.id)
        .bind(new_hash)
        .execute(&state.db)
        .await?;

    Ok(Json(MessageResponse {
        message: "contraseña actualizada".to_string(),
    }))
}
