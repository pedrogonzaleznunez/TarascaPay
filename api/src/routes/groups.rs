//! Grupos: creación, miembros, invitaciones, balances e historial.

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use shared::split::{simplify_debts, NetBalance};
use shared::{
    ActivityResponse, AddMemberRequest, CreateGroupRequest, GroupBalances, GroupDetail,
    GroupResponse, JoinGroupRequest, MemberResponse, MessageResponse, SuggestedTransfer,
    UpdateGroupRequest, UserBrief,
};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::ActivityRow;
use crate::repo;
use crate::state::AppState;
use crate::validate;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/join", post(join_group))
        .route(
            "/groups/{group_id}",
            get(get_group).patch(update_group).delete(delete_group),
        )
        .route("/groups/{group_id}/members", post(add_member))
        .route("/groups/{group_id}/members/{user_id}", delete(remove_member))
        .route("/groups/{group_id}/invite-code", post(regenerate_invite))
        .route("/groups/{group_id}/balances", get(group_balances))
        .route("/groups/{group_id}/activity", get(group_activity))
}

/// Grupo con los agregados que necesita el listado.
#[derive(Debug, FromRow)]
struct GroupStatsRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    kind: String,
    currency: String,
    emoji: String,
    invite_code: String,
    created_by: Uuid,
    archived: bool,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    created_at: DateTime<Utc>,
    role: String,
    member_count: i64,
    expense_count: i64,
    total_spent_cents: i64,
    my_balance_cents: i64,
}

impl From<GroupStatsRow> for GroupResponse {
    fn from(row: GroupStatsRow) -> Self {
        GroupResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            kind: row.kind,
            currency: row.currency.trim().to_string(),
            emoji: row.emoji,
            invite_code: row.invite_code,
            created_by: row.created_by,
            archived: row.archived,
            start_date: row.start_date,
            end_date: row.end_date,
            created_at: row.created_at,
            role: row.role,
            member_count: row.member_count,
            expense_count: row.expense_count,
            total_spent_cents: row.total_spent_cents,
            my_balance_cents: row.my_balance_cents,
        }
    }
}

/// Los agregados por grupo se calculan en la base: evita traer todos los
/// gastos al proceso sólo para sumarlos.
const GROUP_SELECT: &str = r#"
SELECT
    g.id, g.name, g.description, g.kind, g.currency, g.emoji, g.invite_code,
    g.created_by, g.archived, g.start_date, g.end_date, g.created_at,
    gm.role,
    (SELECT COUNT(*) FROM group_members m WHERE m.group_id = g.id)::bigint AS member_count,
    (SELECT COUNT(*) FROM expenses e WHERE e.group_id = g.id AND e.deleted_at IS NULL)::bigint
        AS expense_count,
    (SELECT COALESCE(SUM(e.amount_cents), 0) FROM expenses e
        WHERE e.group_id = g.id AND e.deleted_at IS NULL)::bigint AS total_spent_cents,
    (
          (SELECT COALESCE(SUM(e.amount_cents), 0) FROM expenses e
             WHERE e.group_id = g.id AND e.deleted_at IS NULL AND e.paid_by = $1)
        + (SELECT COALESCE(SUM(st.amount_cents), 0) FROM settlements st
             WHERE st.group_id = g.id AND st.from_user = $1)
        - (SELECT COALESCE(SUM(s.share_cents), 0) FROM expense_splits s
             JOIN expenses e ON e.id = s.expense_id
             WHERE e.group_id = g.id AND e.deleted_at IS NULL AND s.user_id = $1)
        - (SELECT COALESCE(SUM(st.amount_cents), 0) FROM settlements st
             WHERE st.group_id = g.id AND st.to_user = $1)
    )::bigint AS my_balance_cents
FROM groups g
JOIN group_members gm ON gm.group_id = g.id AND gm.user_id = $1
"#;

async fn list_groups(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<GroupResponse>>> {
    // El SQL se arma sólo a partir de constantes; los datos van por bind.
    let rows = sqlx::query_as::<_, GroupStatsRow>(sqlx::AssertSqlSafe(format!(
        "{GROUP_SELECT} ORDER BY g.archived ASC, g.created_at DESC"
    )))
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(GroupResponse::from).collect()))
}

async fn load_group(state: &AppState, group_id: Uuid, user_id: Uuid) -> AppResult<GroupResponse> {
    let row = sqlx::query_as::<_, GroupStatsRow>(sqlx::AssertSqlSafe(format!(
        "{GROUP_SELECT} WHERE g.id = $2"
    )))
        .bind(user_id)
        .bind(group_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("grupo no encontrado o no pertenecés a él"))?;

    Ok(row.into())
}

async fn create_group(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateGroupRequest>,
) -> AppResult<Json<GroupDetail>> {
    let name = validate::non_empty(&payload.name, "el nombre del grupo", 80)?;
    let kind = validate::group_kind(payload.kind.as_deref().unwrap_or("trip"))?;

    // Si no se indica moneda se hereda la del usuario.
    let creator = repo::find_user_by_id(&state.db, user.id).await?;
    let currency = match payload.currency.as_deref() {
        Some(c) => validate::currency(c)?,
        None => creator.currency.trim().to_string(),
    };

    let emoji = payload
        .emoji
        .as_deref()
        .filter(|e| !e.trim().is_empty())
        .unwrap_or(default_emoji(&kind))
        .to_string();

    if let (Some(start), Some(end)) = (payload.start_date, payload.end_date) {
        if end < start {
            return Err(AppError::bad_request(
                "la fecha de fin no puede ser anterior a la de inicio",
            ));
        }
    }

    let mut tx = state.db.begin().await?;

    // El código de invitación es aleatorio: reintentamos ante una colisión.
    let mut group_id: Option<Uuid> = None;
    for _ in 0..5 {
        let code = repo::generate_invite_code();
        let inserted: Result<(Uuid,), sqlx::Error> = sqlx::query_as(
            "INSERT INTO groups (name, description, kind, currency, emoji, invite_code,
                                 created_by, start_date, end_date)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
        )
        .bind(&name)
        .bind(&payload.description)
        .bind(&kind)
        .bind(&currency)
        .bind(&emoji)
        .bind(&code)
        .bind(user.id)
        .bind(payload.start_date)
        .bind(payload.end_date)
        .fetch_one(&mut *tx)
        .await;

        match inserted {
            Ok((id,)) => {
                group_id = Some(id);
                break;
            }
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => continue,
            Err(err) => return Err(err.into()),
        }
    }

    let group_id = group_id
        .ok_or_else(|| AppError::internal("no se pudo generar un código de invitación único"))?;

    sqlx::query("INSERT INTO group_members (group_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(group_id)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    repo::log_activity(
        &state.db,
        Some(group_id),
        user.id,
        "group_created",
        serde_json::json!({ "name": name }),
    )
    .await;

    let group = load_group(&state, group_id, user.id).await?;
    let members = load_members(&state, group_id).await?;

    Ok(Json(GroupDetail { group, members }))
}

fn default_emoji(kind: &str) -> &'static str {
    match kind {
        "trip" => "✈️",
        "home" => "🏠",
        "couple" => "❤️",
        "event" => "🎉",
        _ => "🌎",
    }
}

async fn get_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<GroupDetail>> {
    repo::require_membership(&state.db, group_id, user.id).await?;

    let group = load_group(&state, group_id, user.id).await?;
    let members = load_members(&state, group_id).await?;

    Ok(Json(GroupDetail { group, members }))
}

async fn load_members(state: &AppState, group_id: Uuid) -> AppResult<Vec<MemberResponse>> {
    let balances = repo::group_balances(&state.db, group_id).await?;

    let roles: Vec<(Uuid, String, DateTime<Utc>)> =
        sqlx::query_as("SELECT user_id, role, joined_at FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(&state.db)
            .await?;

    Ok(balances
        .into_iter()
        .map(|b| {
            let meta = roles.iter().find(|(id, _, _)| *id == b.user_id);
            MemberResponse {
                balance_cents: b.net_cents(),
                user: b.user(),
                role: meta
                    .map(|(_, r, _)| r.clone())
                    .unwrap_or_else(|| "member".to_string()),
                joined_at: meta.map(|(_, _, j)| *j).unwrap_or_else(Utc::now),
            }
        })
        .collect())
}

async fn update_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<UpdateGroupRequest>,
) -> AppResult<Json<GroupDetail>> {
    repo::require_owner(&state.db, group_id, user.id).await?;

    let name = payload
        .name
        .as_deref()
        .map(|n| validate::non_empty(n, "el nombre del grupo", 80))
        .transpose()?;
    let kind = payload
        .kind
        .as_deref()
        .map(validate::group_kind)
        .transpose()?;

    // Una descripción vacía la borra; omitir el campo la deja como está.
    let description = payload.description.as_deref().map(str::trim).and_then(|text| {
        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    });
    let clear_description = payload.description.is_some() && description.is_none();

    sqlx::query(
        "UPDATE groups SET
            name        = COALESCE($2, name),
            description = CASE WHEN $9 THEN NULL ELSE COALESCE($3, description) END,
            kind        = COALESCE($4, kind),
            emoji       = COALESCE($5, emoji),
            archived    = COALESCE($6, archived),
            start_date  = COALESCE($7, start_date),
            end_date    = COALESCE($8, end_date),
            updated_at  = NOW()
         WHERE id = $1",
    )
    .bind(group_id)
    .bind(name)
    .bind(description)
    .bind(kind)
    .bind(payload.emoji)
    .bind(payload.archived)
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(clear_description)
    .execute(&state.db)
    .await?;

    let group = load_group(&state, group_id, user.id).await?;
    let members = load_members(&state, group_id).await?;
    Ok(Json(GroupDetail { group, members }))
}

async fn delete_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<MessageResponse>> {
    repo::require_owner(&state.db, group_id, user.id).await?;

    sqlx::query("DELETE FROM groups WHERE id = $1")
        .bind(group_id)
        .execute(&state.db)
        .await?;

    Ok(Json(MessageResponse {
        message: "grupo eliminado".to_string(),
    }))
}

async fn join_group(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<JoinGroupRequest>,
) -> AppResult<Json<GroupDetail>> {
    let code = payload.invite_code.trim().to_uppercase();

    let group: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, name FROM groups WHERE invite_code = $1")
            .bind(&code)
            .fetch_optional(&state.db)
            .await?;

    let (group_id, group_name) =
        group.ok_or_else(|| AppError::not_found("el código de invitación no es válido"))?;

    let already: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM group_members WHERE group_id = $1 AND user_id = $2")
            .bind(group_id)
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;

    if already.is_none() {
        sqlx::query("INSERT INTO group_members (group_id, user_id, role) VALUES ($1, $2, 'member')")
            .bind(group_id)
            .bind(user.id)
            .execute(&state.db)
            .await?;

        repo::log_activity(
            &state.db,
            Some(group_id),
            user.id,
            "member_joined",
            serde_json::json!({ "group": group_name }),
        )
        .await;
    }

    let group = load_group(&state, group_id, user.id).await?;
    let members = load_members(&state, group_id).await?;
    Ok(Json(GroupDetail { group, members }))
}

async fn add_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
    Json(payload): Json<AddMemberRequest>,
) -> AppResult<Json<Vec<MemberResponse>>> {
    repo::require_membership(&state.db, group_id, user.id).await?;

    let email = validate::email(&payload.email)?;
    let invited = repo::find_user_by_email(&state.db, &email)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "no hay ninguna cuenta con ese email; pasale el código de invitación del grupo",
            )
        })?;

    let inserted = sqlx::query(
        "INSERT INTO group_members (group_id, user_id, role) VALUES ($1, $2, 'member')
         ON CONFLICT (group_id, user_id) DO NOTHING",
    )
    .bind(group_id)
    .bind(invited.id)
    .execute(&state.db)
    .await?;

    if inserted.rows_affected() > 0 {
        repo::log_activity(
            &state.db,
            Some(group_id),
            user.id,
            "member_added",
            serde_json::json!({ "member": invited.display_name }),
        )
        .await;
    }

    Ok(Json(load_members(&state, group_id).await?))
}

async fn remove_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path((group_id, target_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Vec<MemberResponse>>> {
    let role = repo::require_membership(&state.db, group_id, user.id).await?;

    // Cualquiera puede irse; sólo el admin puede sacar a otro.
    if target_id != user.id && role != "owner" {
        return Err(AppError::forbidden(
            "sólo el administrador puede sacar a otros miembros",
        ));
    }

    // Un saldo pendiente quedaría huérfano: hay que saldarlo antes.
    let balances = repo::group_balances(&state.db, group_id).await?;
    if let Some(target) = balances.iter().find(|b| b.user_id == target_id) {
        if target.net_cents() != 0 {
            return Err(AppError::conflict(
                "esa persona tiene saldo pendiente en el grupo: salden las cuentas antes de sacarla",
            ));
        }
    }

    // El grupo no puede quedarse sin administrador.
    if target_id == user.id && role == "owner" {
        let other_owners: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM group_members
             WHERE group_id = $1 AND role = 'owner' AND user_id <> $2",
        )
        .bind(group_id)
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;

        let members: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM group_members WHERE group_id = $1")
                .bind(group_id)
                .fetch_one(&state.db)
                .await?;

        if other_owners.0 == 0 && members.0 > 1 {
            return Err(AppError::conflict(
                "sos el único administrador: pasale el rol a otro miembro o eliminá el grupo",
            ));
        }
    }

    sqlx::query("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2")
        .bind(group_id)
        .bind(target_id)
        .execute(&state.db)
        .await?;

    // Si el que se fue era el último, el grupo ya no le sirve a nadie.
    let remaining: (i64,) =
        sqlx::query_as("SELECT COUNT(*)::bigint FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(&state.db)
            .await?;

    if remaining.0 == 0 {
        sqlx::query("DELETE FROM groups WHERE id = $1")
            .bind(group_id)
            .execute(&state.db)
            .await?;
        return Ok(Json(Vec::new()));
    }

    Ok(Json(load_members(&state, group_id).await?))
}

async fn regenerate_invite(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<MessageResponse>> {
    repo::require_owner(&state.db, group_id, user.id).await?;

    for _ in 0..5 {
        let code = repo::generate_invite_code();
        let result = sqlx::query("UPDATE groups SET invite_code = $2, updated_at = NOW() WHERE id = $1")
            .bind(group_id)
            .bind(&code)
            .execute(&state.db)
            .await;

        match result {
            Ok(_) => return Ok(Json(MessageResponse { message: code })),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => continue,
            Err(err) => return Err(err.into()),
        }
    }

    Err(AppError::internal("no se pudo generar un código único"))
}

async fn group_balances(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<GroupBalances>> {
    repo::require_membership(&state.db, group_id, user.id).await?;

    let rows = repo::group_balances(&state.db, group_id).await?;
    let entries = repo::to_balance_entries(&rows);

    let nets: Vec<NetBalance> = rows
        .iter()
        .map(|r| NetBalance {
            user_id: r.user_id,
            net_cents: r.net_cents(),
        })
        .collect();

    let by_id: std::collections::HashMap<Uuid, UserBrief> =
        rows.iter().map(|r| (r.user_id, r.user())).collect();

    let transfers = simplify_debts(&nets)
        .into_iter()
        .filter_map(|t| {
            Some(SuggestedTransfer {
                from: by_id.get(&t.from)?.clone(),
                to: by_id.get(&t.to)?.clone(),
                amount_cents: t.amount_cents,
            })
        })
        .collect();

    let meta: (String, i64) = sqlx::query_as(
        "SELECT g.currency,
                (SELECT COALESCE(SUM(amount_cents), 0) FROM expenses
                  WHERE group_id = g.id AND deleted_at IS NULL)::bigint
         FROM groups g WHERE g.id = $1",
    )
    .bind(group_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(GroupBalances {
        currency: meta.0.trim().to_string(),
        total_spent_cents: meta.1,
        balances: entries,
        transfers,
    }))
}

async fn group_activity(
    State(state): State<AppState>,
    user: AuthUser,
    Path(group_id): Path<Uuid>,
) -> AppResult<Json<Vec<ActivityResponse>>> {
    repo::require_membership(&state.db, group_id, user.id).await?;

    let rows = sqlx::query_as::<_, ActivityRow>(
        r#"
        SELECT a.id, a.group_id, a.kind, a.payload, a.created_at,
               u.id AS actor_id, u.display_name, u.email, u.avatar_url
        FROM activity a
        JOIN users u ON u.id = a.actor_id
        WHERE a.group_id = $1
        ORDER BY a.created_at DESC
        LIMIT 100
        "#,
    )
    .bind(group_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| ActivityResponse {
                id: r.id,
                group_id: r.group_id,
                actor: UserBrief {
                    id: r.actor_id,
                    display_name: r.display_name,
                    email: r.email,
                    avatar_url: r.avatar_url,
                },
                kind: r.kind,
                payload: r.payload,
                created_at: r.created_at,
            })
            .collect(),
    ))
}
