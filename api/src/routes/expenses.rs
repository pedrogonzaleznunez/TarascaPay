//! Gastos: alta, edición, borrado, comentarios y categorías.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use shared::split::{compute_shares, Participant, SplitType};
use shared::{
    CategoryResponse, CommentResponse, CreateCommentRequest, CreateExpenseRequest, ExpenseQuery,
    ExpenseResponse, MessageResponse, SplitInput, UpdateExpenseRequest,
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::{CategoryRow, CommentRow};
use crate::repo::{self, ExpenseFilter};
use crate::state::AppState;
use crate::validate;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list_categories))
        .route("/expenses", get(list_expenses).post(create_expense))
        .route(
            "/expenses/{expense_id}",
            get(get_expense).patch(update_expense).delete(delete_expense),
        )
        .route(
            "/expenses/{expense_id}/comments",
            get(list_comments).post(add_comment),
        )
}

async fn list_categories(State(state): State<AppState>) -> AppResult<Json<Vec<CategoryResponse>>> {
    let rows = sqlx::query_as::<_, CategoryRow>(
        "SELECT id, slug, name, icon FROM categories ORDER BY position, name",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(CategoryResponse::from).collect()))
}

async fn list_expenses(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ExpenseQuery>,
) -> AppResult<Json<Vec<ExpenseResponse>>> {
    let mut filter = ExpenseFilter {
        category_id: query.category_id,
        search: query.search,
        from: query.from,
        to: query.to,
        limit: query.limit.unwrap_or(50),
        offset: query.offset.unwrap_or(0),
        ..Default::default()
    };

    match (query.group_id, query.personal.unwrap_or(false)) {
        (Some(group_id), _) => {
            repo::require_membership(&state.db, group_id, user.id).await?;
            filter.group_id = Some(group_id);
        }
        (None, true) => filter.personal_of = Some(user.id),
        // Sin filtros: todo lo que el usuario puede ver.
        (None, false) => filter.visible_to = Some(user.id),
    }

    Ok(Json(repo::fetch_expenses(&state.db, filter, user.id).await?))
}

async fn get_expense(
    State(state): State<AppState>,
    user: AuthUser,
    Path(expense_id): Path<Uuid>,
) -> AppResult<Json<ExpenseResponse>> {
    Ok(Json(
        repo::fetch_expense_for_user(&state.db, expense_id, user.id).await?,
    ))
}

async fn create_expense(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateExpenseRequest>,
) -> AppResult<Json<ExpenseResponse>> {
    let description = validate::non_empty(&payload.description, "la descripción", 200)?;
    let amount_cents = validate::amount_cents(payload.amount_cents)?;
    let split_type = SplitType::parse(&payload.split_type)?;

    // Un gasto personal es, en los hechos, un gasto de un solo participante:
    // así el resto del sistema (balances, estadísticas) lo trata igual.
    let (currency, paid_by, participants) = match payload.group_id {
        Some(group_id) => {
            repo::require_membership(&state.db, group_id, user.id).await?;
            let members = group_member_ids(&state.db, group_id).await?;

            let paid_by = payload.paid_by.unwrap_or(user.id);
            if !members.contains(&paid_by) {
                return Err(AppError::bad_request(
                    "quien pagó tiene que ser miembro del grupo",
                ));
            }

            let participants = resolve_participants(&payload.splits, &members, split_type)?;

            let currency: (String,) = sqlx::query_as("SELECT currency FROM groups WHERE id = $1")
                .bind(group_id)
                .fetch_one(&state.db)
                .await?;

            (currency.0.trim().to_string(), paid_by, participants)
        }
        None => {
            let me = repo::find_user_by_id(&state.db, user.id).await?;
            let currency = match payload.currency.as_deref() {
                Some(c) => validate::currency(c)?,
                None => me.currency.trim().to_string(),
            };
            (
                currency,
                user.id,
                vec![Participant {
                    user_id: user.id,
                    value: amount_cents,
                }],
            )
        }
    };

    // En un gasto personal el único participante carga el total exacto.
    let effective_split = if payload.group_id.is_none() {
        SplitType::Exact
    } else {
        split_type
    };

    let shares = compute_shares(amount_cents, effective_split, &participants)?;

    let category_id = resolve_category(&state.db, payload.category_id).await?;
    let expense_date = payload
        .expense_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let mut tx = state.db.begin().await?;

    let (expense_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO expenses (group_id, created_by, paid_by, description, notes, amount_cents,
                               currency, category_id, expense_date, split_type)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(payload.group_id)
    .bind(user.id)
    .bind(paid_by)
    .bind(&description)
    .bind(&payload.notes)
    .bind(amount_cents)
    .bind(&currency)
    .bind(category_id)
    .bind(expense_date)
    .bind(effective_split.as_str())
    .fetch_one(&mut *tx)
    .await?;

    insert_splits(&mut tx, expense_id, &shares).await?;
    link_attachments(&mut tx, expense_id, user.id, &payload.attachment_ids).await?;

    tx.commit().await?;

    repo::log_activity(
        &state.db,
        payload.group_id,
        user.id,
        "expense_created",
        serde_json::json!({
            "expense_id": expense_id,
            "description": description,
            "amount_cents": amount_cents,
        }),
    )
    .await;

    Ok(Json(
        repo::fetch_expense_for_user(&state.db, expense_id, user.id).await?,
    ))
}

async fn update_expense(
    State(state): State<AppState>,
    user: AuthUser,
    Path(expense_id): Path<Uuid>,
    Json(payload): Json<UpdateExpenseRequest>,
) -> AppResult<Json<ExpenseResponse>> {
    let current = repo::fetch_expense_for_user(&state.db, expense_id, user.id).await?;
    ensure_can_modify(&state.db, &current, user.id).await?;

    let description = match payload.description.as_deref() {
        Some(d) => validate::non_empty(d, "la descripción", 200)?,
        None => current.description.clone(),
    };
    let amount_cents = match payload.amount_cents {
        Some(a) => validate::amount_cents(a)?,
        None => current.amount_cents,
    };

    let split_type = match payload.split_type.as_deref() {
        Some(s) => SplitType::parse(s)?,
        None => SplitType::parse(&current.split_type)?,
    };

    let paid_by = payload.paid_by.unwrap_or(current.paid_by.id);

    // El reparto se recalcula siempre: cambiar el importe sin rehacerlo dejaría
    // partes que no suman el total.
    let (effective_split, participants) = match current.group_id {
        Some(group_id) => {
            let members = group_member_ids(&state.db, group_id).await?;
            if !members.contains(&paid_by) {
                return Err(AppError::bad_request(
                    "quien pagó tiene que ser miembro del grupo",
                ));
            }

            let inputs: Vec<SplitInput> = match payload.splits {
                Some(ref splits) => splits.clone(),
                // Sin repartos nuevos: se conservan los participantes actuales.
                None => current
                    .splits
                    .iter()
                    .map(|s| SplitInput {
                        user_id: s.user.id,
                        value: s.split_value,
                    })
                    .collect(),
            };

            (split_type, resolve_participants(&inputs, &members, split_type)?)
        }
        None => (
            SplitType::Exact,
            vec![Participant {
                user_id: user.id,
                value: amount_cents,
            }],
        ),
    };

    let shares = compute_shares(amount_cents, effective_split, &participants)?;

    let category_id = match payload.category_id {
        Some(id) => resolve_category(&state.db, Some(id)).await?,
        None => current.category.id,
    };

    let mut tx = state.db.begin().await?;

    // Una nota vacía significa "borrar la nota"; omitir el campo la deja como está.
    let notes = match payload.notes.as_deref().map(str::trim) {
        Some("") => None,
        Some(text) => Some(text.to_string()),
        None => current.notes.clone(),
    };

    sqlx::query(
        "UPDATE expenses SET
            description  = $2,
            notes        = $3,
            amount_cents = $4,
            category_id  = $5,
            expense_date = COALESCE($6, expense_date),
            paid_by      = $7,
            split_type   = $8,
            updated_at   = NOW()
         WHERE id = $1",
    )
    .bind(expense_id)
    .bind(&description)
    .bind(&notes)
    .bind(amount_cents)
    .bind(category_id)
    .bind(payload.expense_date)
    .bind(paid_by)
    .bind(effective_split.as_str())
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM expense_splits WHERE expense_id = $1")
        .bind(expense_id)
        .execute(&mut *tx)
        .await?;

    insert_splits(&mut tx, expense_id, &shares).await?;

    if let Some(ref attachment_ids) = payload.attachment_ids {
        // Se desvinculan sólo los que el usuario sacó de la lista. Borrar todos
        // y re-vincular dejaría huérfanos los comprobantes que subió otra
        // persona, porque `link_attachments` sólo toca los propios.
        sqlx::query(
            "UPDATE attachments SET expense_id = NULL
             WHERE expense_id = $1 AND NOT (id = ANY($2))",
        )
        .bind(expense_id)
        .bind(attachment_ids)
        .execute(&mut *tx)
        .await?;

        link_attachments(&mut tx, expense_id, user.id, attachment_ids).await?;
    }

    tx.commit().await?;

    repo::log_activity(
        &state.db,
        current.group_id,
        user.id,
        "expense_updated",
        serde_json::json!({ "expense_id": expense_id, "description": description }),
    )
    .await;

    Ok(Json(
        repo::fetch_expense_for_user(&state.db, expense_id, user.id).await?,
    ))
}

async fn delete_expense(
    State(state): State<AppState>,
    user: AuthUser,
    Path(expense_id): Path<Uuid>,
) -> AppResult<Json<MessageResponse>> {
    let current = repo::fetch_expense_for_user(&state.db, expense_id, user.id).await?;
    ensure_can_modify(&state.db, &current, user.id).await?;

    // Borrado lógico: el historial del grupo sigue teniendo sentido.
    sqlx::query("UPDATE expenses SET deleted_at = NOW() WHERE id = $1")
        .bind(expense_id)
        .execute(&state.db)
        .await?;

    repo::log_activity(
        &state.db,
        current.group_id,
        user.id,
        "expense_deleted",
        serde_json::json!({ "description": current.description }),
    )
    .await;

    Ok(Json(MessageResponse {
        message: "gasto eliminado".to_string(),
    }))
}

async fn list_comments(
    State(state): State<AppState>,
    user: AuthUser,
    Path(expense_id): Path<Uuid>,
) -> AppResult<Json<Vec<CommentResponse>>> {
    repo::fetch_expense_for_user(&state.db, expense_id, user.id).await?;

    let rows = sqlx::query_as::<_, CommentRow>(
        "SELECT c.id, c.body, c.created_at, u.id AS user_id, u.display_name, u.email, u.avatar_url
         FROM comments c JOIN users u ON u.id = c.user_id
         WHERE c.expense_id = $1 ORDER BY c.created_at",
    )
    .bind(expense_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(CommentResponse::from).collect()))
}

async fn add_comment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(expense_id): Path<Uuid>,
    Json(payload): Json<CreateCommentRequest>,
) -> AppResult<Json<CommentResponse>> {
    repo::fetch_expense_for_user(&state.db, expense_id, user.id).await?;
    let body = validate::non_empty(&payload.body, "el comentario", 2000)?;

    let (id,): (Uuid,) =
        sqlx::query_as("INSERT INTO comments (expense_id, user_id, body) VALUES ($1, $2, $3) RETURNING id")
            .bind(expense_id)
            .bind(user.id)
            .bind(&body)
            .fetch_one(&state.db)
            .await?;

    let row = sqlx::query_as::<_, CommentRow>(
        "SELECT c.id, c.body, c.created_at, u.id AS user_id, u.display_name, u.email, u.avatar_url
         FROM comments c JOIN users u ON u.id = c.user_id WHERE c.id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(row.into()))
}

// ---------------------------------------------------------------------------
// Auxiliares
// ---------------------------------------------------------------------------

async fn group_member_ids(db: &PgPool, group_id: Uuid) -> AppResult<Vec<Uuid>> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT m.user_id FROM group_members m
         JOIN users u ON u.id = m.user_id
         WHERE m.group_id = $1 ORDER BY u.display_name",
    )
    .bind(group_id)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Convierte los repartos pedidos en participantes validados.
///
/// Sin repartos explícitos se asume "entre todos, en partes iguales".
fn resolve_participants(
    splits: &[SplitInput],
    members: &[Uuid],
    split_type: SplitType,
) -> AppResult<Vec<Participant>> {
    if splits.is_empty() {
        if split_type != SplitType::Equal {
            return Err(AppError::bad_request(
                "indicá el reparto de cada participante",
            ));
        }
        return Ok(members
            .iter()
            .map(|&user_id| Participant { user_id, value: 0 })
            .collect());
    }

    for split in splits {
        if !members.contains(&split.user_id) {
            return Err(AppError::bad_request(
                "hay participantes que no son miembros del grupo",
            ));
        }
    }

    Ok(splits
        .iter()
        .map(|s| Participant {
            user_id: s.user_id,
            value: s.value,
        })
        .collect())
}

async fn resolve_category(db: &PgPool, category_id: Option<i32>) -> AppResult<i32> {
    match category_id {
        Some(id) => {
            let exists: Option<(i32,)> = sqlx::query_as("SELECT id FROM categories WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?;
            exists
                .map(|(id,)| id)
                .ok_or_else(|| AppError::bad_request("la categoría no existe"))
        }
        None => {
            let (id,): (i32,) = sqlx::query_as("SELECT id FROM categories WHERE slug = 'general'")
                .fetch_one(db)
                .await?;
            Ok(id)
        }
    }
}

async fn insert_splits(
    tx: &mut Transaction<'_, Postgres>,
    expense_id: Uuid,
    shares: &[shared::split::Share],
) -> AppResult<()> {
    for share in shares {
        sqlx::query(
            "INSERT INTO expense_splits (expense_id, user_id, share_cents, split_value)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(expense_id)
        .bind(share.user_id)
        .bind(share.share_cents)
        .bind(share.split_value)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Vincula adjuntos previamente subidos, sólo si los subió el mismo usuario y
/// todavía no pertenecen a otro gasto.
async fn link_attachments(
    tx: &mut Transaction<'_, Postgres>,
    expense_id: Uuid,
    user_id: Uuid,
    attachment_ids: &[Uuid],
) -> AppResult<()> {
    if attachment_ids.is_empty() {
        return Ok(());
    }

    sqlx::query(
        "UPDATE attachments SET expense_id = $1
         WHERE id = ANY($2) AND uploaded_by = $3 AND expense_id IS NULL",
    )
    .bind(expense_id)
    .bind(attachment_ids)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Puede editar/borrar quien lo creó, quien pagó, o el admin del grupo.
async fn ensure_can_modify(db: &PgPool, expense: &ExpenseResponse, user_id: Uuid) -> AppResult<()> {
    if expense.created_by == user_id || expense.paid_by.id == user_id {
        return Ok(());
    }

    if let Some(group_id) = expense.group_id {
        if repo::require_membership(db, group_id, user_id).await? == "owner" {
            return Ok(());
        }
    }

    Err(AppError::forbidden(
        "sólo quien cargó el gasto, quien lo pagó o el admin del grupo pueden modificarlo",
    ))
}
