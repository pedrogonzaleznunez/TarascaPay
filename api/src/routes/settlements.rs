//! Pagos entre miembros para saldar cuentas.

use axum::extract::{Path, State};
use axum::routing::{delete, get};
use axum::{Json, Router};
use shared::{CreateSettlementRequest, MessageResponse, SettlementResponse};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::SettlementRow;
use crate::repo;
use crate::state::AppState;
use crate::validate;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/groups/{group_id}/settlements",
            get(list_settlements).post(create_settlement),
        )
        .route("/settlements/{settlement_id}", delete(delete_settlement))
}

const SETTLEMENT_SELECT: &str = r#"
SELECT s.id, s.group_id, s.amount_cents, s.currency, s.note, s.settled_at, s.created_at,
       f.id AS from_id, f.display_name AS from_name, f.email AS from_email,
       f.avatar_url AS from_avatar,
       t.id AS to_id, t.display_name AS to_name, t.email AS to_email,
       t.avatar_url AS to_avatar
FROM settlements s
JOIN users f ON f.id = s.from_user
JOIN users t ON t.id = s.to_user
"#;

async fn list_settlements(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<Vec<SettlementResponse>>> {
    repo::require_membership(&state.db, group_id, user.id).await?;

    // El SQL se arma sólo a partir de constantes; los datos van por bind.
    let rows = sqlx::query_as::<_, SettlementRow>(sqlx::AssertSqlSafe(format!(
        "{SETTLEMENT_SELECT} WHERE s.group_id = $1 ORDER BY s.settled_at DESC, s.created_at DESC"
    )))
    .bind(group_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn create_settlement(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<CreateSettlementRequest>,
) -> AppResult<Json<SettlementResponse>> {
    repo::require_membership(&state.db, group_id, user.id).await?;

    let amount_cents = validate::amount_cents(payload.amount_cents)?;

    if payload.from_user == payload.to_user {
        return Err(AppError::bad_request(
            "el pago tiene que ser entre dos personas distintas",
        ));
    }

    // Ambas partes deben pertenecer al grupo.
    for participant in [payload.from_user, payload.to_user] {
        repo::require_membership(&state.db, group_id, participant)
            .await
            .map_err(|_| {
                AppError::bad_request("ambas personas tienen que ser miembros del grupo")
            })?;
    }

    let (currency,): (String,) = sqlx::query_as("SELECT currency FROM groups WHERE id = $1")
        .bind(group_id)
        .fetch_one(&state.db)
        .await?;

    let settled_at = payload
        .settled_at
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO settlements (group_id, from_user, to_user, amount_cents, currency, note,
                                  created_by, settled_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(group_id)
    .bind(payload.from_user)
    .bind(payload.to_user)
    .bind(amount_cents)
    .bind(currency.trim())
    .bind(&payload.note)
    .bind(user.id)
    .bind(settled_at)
    .fetch_one(&state.db)
    .await?;

    repo::log_activity(
        &state.db,
        Some(group_id),
        user.id,
        "settlement_created",
        serde_json::json!({ "amount_cents": amount_cents }),
    )
    .await;

    let row = sqlx::query_as::<_, SettlementRow>(sqlx::AssertSqlSafe(format!(
        "{SETTLEMENT_SELECT} WHERE s.id = $1"
    )))
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(row.into()))
}

async fn delete_settlement(
    State(state): State<AppState>,
    user: AuthUser,
    Path(settlement_id): Path<Uuid>,
) -> AppResult<Json<MessageResponse>> {
    let row: Option<(Option<Uuid>, Uuid, Uuid, Uuid)> = sqlx::query_as(
        "SELECT group_id, created_by, from_user, to_user FROM settlements WHERE id = $1",
    )
    .bind(settlement_id)
    .fetch_optional(&state.db)
    .await?;

    let (group_id, created_by, from_user, to_user) =
        row.ok_or_else(|| AppError::not_found("el pago no existe"))?;

    let involved = created_by == user.id || from_user == user.id || to_user == user.id;
    let is_owner = match group_id {
        Some(gid) => repo::require_membership(&state.db, gid, user.id).await? == "owner",
        None => false,
    };

    if !involved && !is_owner {
        return Err(AppError::forbidden(
            "sólo quien participó del pago o el admin del grupo pueden borrarlo",
        ));
    }

    sqlx::query("DELETE FROM settlements WHERE id = $1")
        .bind(settlement_id)
        .execute(&state.db)
        .await?;

    Ok(Json(MessageResponse {
        message: "pago eliminado".to_string(),
    }))
}
