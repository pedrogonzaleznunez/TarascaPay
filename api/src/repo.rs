//! Consultas reutilizadas por varios handlers.

use std::collections::HashMap;

use shared::{
    AttachmentResponse, BalanceEntry, ExpenseResponse, ExpenseSplitResponse, UserBrief,
};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{AttachmentRow, BalanceRow, ExpenseRow, SplitRow, UserRow};

/// Columnas de un gasto ya unidas a categoría, pagador y grupo.
const EXPENSE_SELECT: &str = r#"
SELECT
    e.id,
    e.group_id,
    g.name              AS group_name,
    e.description,
    e.notes,
    e.amount_cents,
    e.currency,
    e.expense_date,
    e.split_type,
    e.created_by,
    e.created_at,
    c.id                AS category_id,
    c.slug              AS category_slug,
    c.name              AS category_name,
    c.icon              AS category_icon,
    p.id                AS payer_id,
    p.display_name      AS payer_name,
    p.email             AS payer_email,
    p.avatar_url        AS payer_avatar
FROM expenses e
JOIN categories c ON c.id = e.category_id
JOIN users p      ON p.id = e.paid_by
LEFT JOIN groups g ON g.id = e.group_id
"#;

pub async fn find_user_by_id(db: &PgPool, id: Uuid) -> AppResult<UserRow> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, display_name, avatar_url, currency, created_at
         FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("usuario no encontrado"))
}

pub async fn find_user_by_email(db: &PgPool, email: &str) -> AppResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password_hash, display_name, avatar_url, currency, created_at
         FROM users WHERE email = $1",
    )
    .bind(email.trim().to_lowercase())
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Verifica que el usuario pertenezca al grupo y devuelve su rol.
pub async fn require_membership(db: &PgPool, group_id: Uuid, user_id: Uuid) -> AppResult<String> {
    let role: Option<(String,)> =
        sqlx::query_as("SELECT role FROM group_members WHERE group_id = $1 AND user_id = $2")
            .bind(group_id)
            .bind(user_id)
            .fetch_optional(db)
            .await?;

    role.map(|r| r.0)
        .ok_or_else(|| AppError::forbidden("no pertenecés a este grupo"))
}

pub async fn require_owner(db: &PgPool, group_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let role = require_membership(db, group_id, user_id).await?;
    if role != "owner" {
        return Err(AppError::forbidden(
            "sólo el administrador del grupo puede hacer esto",
        ));
    }
    Ok(())
}

/// Saldos de todos los miembros de un grupo.
///
/// `paid_cents` suma lo que la persona puso (gastos que pagó + pagos que hizo
/// para saldar). `owed_cents` suma lo que le tocaba (su parte de cada gasto +
/// pagos que recibió). El neto es la diferencia.
pub async fn group_balances(db: &PgPool, group_id: Uuid) -> AppResult<Vec<BalanceRow>> {
    let rows = sqlx::query_as::<_, BalanceRow>(
        r#"
        SELECT
            m.user_id,
            u.display_name,
            u.email,
            u.avatar_url,
            (COALESCE(paid.total, 0) + COALESCE(sent.total, 0))::bigint AS paid_cents,
            (COALESCE(owed.total, 0) + COALESCE(received.total, 0))::bigint AS owed_cents
        FROM group_members m
        JOIN users u ON u.id = m.user_id
        LEFT JOIN (
            SELECT paid_by AS uid, SUM(amount_cents) AS total
            FROM expenses
            WHERE group_id = $1 AND deleted_at IS NULL
            GROUP BY paid_by
        ) paid ON paid.uid = m.user_id
        LEFT JOIN (
            SELECT s.user_id AS uid, SUM(s.share_cents) AS total
            FROM expense_splits s
            JOIN expenses e ON e.id = s.expense_id
            WHERE e.group_id = $1 AND e.deleted_at IS NULL
            GROUP BY s.user_id
        ) owed ON owed.uid = m.user_id
        LEFT JOIN (
            SELECT from_user AS uid, SUM(amount_cents) AS total
            FROM settlements WHERE group_id = $1 GROUP BY from_user
        ) sent ON sent.uid = m.user_id
        LEFT JOIN (
            SELECT to_user AS uid, SUM(amount_cents) AS total
            FROM settlements WHERE group_id = $1 GROUP BY to_user
        ) received ON received.uid = m.user_id
        WHERE m.group_id = $1
        ORDER BY u.display_name
        "#,
    )
    .bind(group_id)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

pub fn to_balance_entries(rows: &[BalanceRow]) -> Vec<BalanceEntry> {
    rows.iter()
        .map(|r| BalanceEntry {
            user: r.user(),
            net_cents: r.net_cents(),
            paid_cents: r.paid_cents,
            owed_cents: r.owed_cents,
        })
        .collect()
}

/// Filtros del listado de gastos.
pub struct ExpenseFilter {
    /// Gastos de este grupo.
    pub group_id: Option<Uuid>,
    /// Sólo gastos personales del usuario.
    pub personal_of: Option<Uuid>,
    /// Todos los gastos visibles para el usuario (sus grupos + sus personales).
    pub visible_to: Option<Uuid>,
    pub expense_id: Option<Uuid>,
    pub category_id: Option<i32>,
    pub search: Option<String>,
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for ExpenseFilter {
    fn default() -> Self {
        Self {
            group_id: None,
            personal_of: None,
            visible_to: None,
            expense_id: None,
            category_id: None,
            search: None,
            from: None,
            to: None,
            limit: 50,
            offset: 0,
        }
    }
}

/// Carga gastos ya hidratados con sus repartos y adjuntos.
///
/// `viewer` se usa para calcular `my_net_cents` en cada gasto.
pub async fn fetch_expenses(
    db: &PgPool,
    filter: ExpenseFilter,
    viewer: Uuid,
) -> AppResult<Vec<ExpenseResponse>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(EXPENSE_SELECT);
    qb.push(" WHERE e.deleted_at IS NULL");

    if let Some(id) = filter.expense_id {
        qb.push(" AND e.id = ").push_bind(id);
    }
    if let Some(group_id) = filter.group_id {
        qb.push(" AND e.group_id = ").push_bind(group_id);
    }
    if let Some(owner) = filter.personal_of {
        qb.push(" AND e.group_id IS NULL AND e.created_by = ")
            .push_bind(owner);
    }
    if let Some(user) = filter.visible_to {
        qb.push(" AND (")
            .push("(e.group_id IS NULL AND e.created_by = ")
            .push_bind(user)
            .push(") OR e.group_id IN (SELECT group_id FROM group_members WHERE user_id = ")
            .push_bind(user)
            .push("))");
    }
    if let Some(category_id) = filter.category_id {
        qb.push(" AND e.category_id = ").push_bind(category_id);
    }
    if let Some(search) = filter.search.as_ref().filter(|s| !s.trim().is_empty()) {
        let pattern = format!("%{}%", search.trim());
        qb.push(" AND (e.description ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR e.notes ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if let Some(from) = filter.from {
        qb.push(" AND e.expense_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        qb.push(" AND e.expense_date <= ").push_bind(to);
    }

    qb.push(" ORDER BY e.expense_date DESC, e.created_at DESC");
    qb.push(" LIMIT ").push_bind(filter.limit.clamp(1, 500));
    qb.push(" OFFSET ").push_bind(filter.offset.max(0));

    let rows: Vec<ExpenseRow> = qb.build_query_as().fetch_all(db).await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let (splits, attachments, comment_counts) = tokio::try_join!(
        load_splits(db, &ids),
        load_attachments(db, &ids),
        load_comment_counts(db, &ids),
    )?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let expense_splits = splits.get(&row.id).cloned().unwrap_or_default();

            // Lo que el usuario puso de más (o de menos) en este gasto.
            let my_share: i64 = expense_splits
                .iter()
                .filter(|s| s.user.id == viewer)
                .map(|s| s.share_cents)
                .sum();
            let my_paid = if row.payer_id == viewer {
                row.amount_cents
            } else {
                0
            };

            ExpenseResponse {
                category: row.category(),
                paid_by: row.payer(),
                my_net_cents: my_paid - my_share,
                comment_count: comment_counts.get(&row.id).copied().unwrap_or(0),
                attachments: attachments.get(&row.id).cloned().unwrap_or_default(),
                splits: expense_splits,
                id: row.id,
                group_id: row.group_id,
                group_name: row.group_name,
                description: row.description,
                notes: row.notes,
                amount_cents: row.amount_cents,
                currency: row.currency.trim().to_string(),
                expense_date: row.expense_date,
                split_type: row.split_type,
                created_by: row.created_by,
                created_at: row.created_at,
            }
        })
        .collect())
}

async fn load_splits(
    db: &PgPool,
    expense_ids: &[Uuid],
) -> AppResult<HashMap<Uuid, Vec<ExpenseSplitResponse>>> {
    let rows = sqlx::query_as::<_, SplitRow>(
        r#"
        SELECT s.expense_id, s.share_cents, s.split_value,
               u.id AS user_id, u.display_name, u.email, u.avatar_url
        FROM expense_splits s
        JOIN users u ON u.id = s.user_id
        WHERE s.expense_id = ANY($1)
        ORDER BY u.display_name
        "#,
    )
    .bind(expense_ids)
    .fetch_all(db)
    .await?;

    let mut map: HashMap<Uuid, Vec<ExpenseSplitResponse>> = HashMap::new();
    for row in rows {
        map.entry(row.expense_id)
            .or_default()
            .push(ExpenseSplitResponse {
                user: UserBrief {
                    id: row.user_id,
                    display_name: row.display_name,
                    email: row.email,
                    avatar_url: row.avatar_url,
                },
                share_cents: row.share_cents,
                split_value: row.split_value,
            });
    }
    Ok(map)
}

async fn load_attachments(
    db: &PgPool,
    expense_ids: &[Uuid],
) -> AppResult<HashMap<Uuid, Vec<AttachmentResponse>>> {
    let rows = sqlx::query_as::<_, AttachmentRow>(
        "SELECT id, expense_id, uploaded_by, file_name, stored_name, mime_type, size_bytes, created_at
         FROM attachments WHERE expense_id = ANY($1) ORDER BY created_at",
    )
    .bind(expense_ids)
    .fetch_all(db)
    .await?;

    let mut map: HashMap<Uuid, Vec<AttachmentResponse>> = HashMap::new();
    for row in rows {
        if let Some(expense_id) = row.expense_id {
            map.entry(expense_id).or_default().push(row.into_response());
        }
    }
    Ok(map)
}

async fn load_comment_counts(db: &PgPool, expense_ids: &[Uuid]) -> AppResult<HashMap<Uuid, i64>> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT expense_id, COUNT(*)::bigint FROM comments
         WHERE expense_id = ANY($1) GROUP BY expense_id",
    )
    .bind(expense_ids)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().collect())
}

/// Carga un gasto verificando que el usuario tenga permiso para verlo.
pub async fn fetch_expense_for_user(
    db: &PgPool,
    expense_id: Uuid,
    viewer: Uuid,
) -> AppResult<ExpenseResponse> {
    let expenses = fetch_expenses(
        db,
        ExpenseFilter {
            expense_id: Some(expense_id),
            limit: 1,
            ..Default::default()
        },
        viewer,
    )
    .await?;

    let expense = expenses
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("gasto no encontrado"))?;

    match expense.group_id {
        Some(group_id) => {
            require_membership(db, group_id, viewer).await?;
        }
        None if expense.created_by != viewer => {
            return Err(AppError::forbidden("este gasto personal no es tuyo"));
        }
        None => {}
    }

    Ok(expense)
}

/// Registra un evento en el historial del grupo. Nunca hace fallar la
/// operación principal: si el log falla, se deja constancia y se sigue.
pub async fn log_activity(
    db: &PgPool,
    group_id: Option<Uuid>,
    actor_id: Uuid,
    kind: &str,
    payload: serde_json::Value,
) {
    let result = sqlx::query(
        "INSERT INTO activity (group_id, actor_id, kind, payload) VALUES ($1, $2, $3, $4)",
    )
    .bind(group_id)
    .bind(actor_id)
    .bind(kind)
    .bind(payload)
    .execute(db)
    .await;

    if let Err(err) = result {
        tracing::warn!(error = %err, kind, "no se pudo registrar la actividad");
    }
}

/// Genera un código de invitación corto y legible (sin caracteres ambiguos).
pub fn generate_invite_code() -> String {
    use rand::Rng;
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}
