//! Panel principal y estadísticas.

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use shared::split::{simplify_debts, NetBalance};
use shared::{
    CategoryBreakdown, CategoryResponse, DashboardSummary, DebtSummary, MonthlyPoint,
    StatsResponse, UserBrief,
};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::repo::{self, ExpenseFilter};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(dashboard))
        .route("/stats", get(stats))
}

async fn dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<DashboardSummary>> {
    let me = repo::find_user_by_id(&state.db, user.id).await?;

    let group_ids: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT g.id FROM groups g
         JOIN group_members m ON m.group_id = g.id
         WHERE m.user_id = $1 AND g.archived = FALSE",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    // Deuda neta con cada persona, acumulada sobre todos los grupos activos.
    //
    // Nota: si tenés grupos en monedas distintas los importes se suman sin
    // convertir. El detalle exacto por grupo está siempre en su propia pantalla.
    let mut per_person: HashMap<Uuid, (UserBrief, i64)> = HashMap::new();

    for (group_id,) in &group_ids {
        let rows = repo::group_balances(&state.db, *group_id).await?;
        let nets: Vec<NetBalance> = rows
            .iter()
            .map(|r| NetBalance {
                user_id: r.user_id,
                net_cents: r.net_cents(),
            })
            .collect();

        let people: HashMap<Uuid, UserBrief> = rows.iter().map(|r| (r.user_id, r.user())).collect();

        for transfer in simplify_debts(&nets) {
            // Sólo interesan las transferencias en las que participo.
            let (other, delta) = if transfer.from == user.id {
                (transfer.to, -transfer.amount_cents)
            } else if transfer.to == user.id {
                (transfer.from, transfer.amount_cents)
            } else {
                continue;
            };

            if let Some(brief) = people.get(&other) {
                let entry = per_person
                    .entry(other)
                    .or_insert_with(|| (brief.clone(), 0));
                entry.1 += delta;
            }
        }
    }

    let mut debts: Vec<DebtSummary> = per_person
        .into_values()
        .filter(|(_, net)| *net != 0)
        .map(|(user, net_cents)| DebtSummary { user, net_cents })
        .collect();

    // Primero a quien más le debés / quien más te debe.
    debts.sort_by_key(|d| -d.net_cents.abs());

    let you_are_owed_cents = debts.iter().filter(|d| d.net_cents > 0).map(|d| d.net_cents).sum();
    let you_owe_cents: i64 = debts
        .iter()
        .filter(|d| d.net_cents < 0)
        .map(|d| -d.net_cents)
        .sum();

    let (personal_this_month_cents,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM expenses
         WHERE created_by = $1 AND group_id IS NULL AND deleted_at IS NULL
           AND expense_date >= date_trunc('month', CURRENT_DATE)",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    // De los gastos grupales sólo cuenta la parte que me tocó.
    let (group_this_month_cents,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(s.share_cents), 0)::bigint
         FROM expense_splits s
         JOIN expenses e ON e.id = s.expense_id
         WHERE s.user_id = $1 AND e.group_id IS NOT NULL AND e.deleted_at IS NULL
           AND e.expense_date >= date_trunc('month', CURRENT_DATE)",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    let recent_expenses = repo::fetch_expenses(
        &state.db,
        ExpenseFilter {
            visible_to: Some(user.id),
            limit: 8,
            ..Default::default()
        },
        user.id,
    )
    .await?;

    Ok(Json(DashboardSummary {
        currency: me.currency.trim().to_string(),
        you_are_owed_cents,
        you_owe_cents,
        net_cents: you_are_owed_cents - you_owe_cents,
        personal_this_month_cents,
        group_this_month_cents,
        active_groups: group_ids.len() as i64,
        debts,
        recent_expenses,
    }))
}

#[derive(Debug, Deserialize)]
struct StatsQuery {
    #[serde(default)]
    group_id: Option<Uuid>,
    #[serde(default)]
    personal: Option<bool>,
    /// Cantidad de meses hacia atrás en la serie temporal.
    #[serde(default)]
    months: Option<i32>,
}

#[derive(Debug, FromRow)]
struct CategoryStatRow {
    id: i32,
    slug: String,
    name: String,
    icon: String,
    total_cents: i64,
    expense_count: i64,
}

#[derive(Debug, FromRow)]
struct MonthRow {
    month: String,
    total_cents: i64,
}

/// El scope define qué gastos entran en las estadísticas. Se resuelve una vez
/// y se reutiliza en las dos consultas de abajo.
enum Scope {
    Group(Uuid),
    Personal,
    Everything,
}

async fn stats(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<StatsQuery>,
) -> AppResult<Json<StatsResponse>> {
    let me = repo::find_user_by_id(&state.db, user.id).await?;

    let (scope, currency) = match (query.group_id, query.personal.unwrap_or(false)) {
        (Some(group_id), _) => {
            repo::require_membership(&state.db, group_id, user.id).await?;
            let (currency,): (String,) = sqlx::query_as("SELECT currency FROM groups WHERE id = $1")
                .bind(group_id)
                .fetch_one(&state.db)
                .await?;
            (Scope::Group(group_id), currency.trim().to_string())
        }
        (None, true) => (Scope::Personal, me.currency.trim().to_string()),
        (None, false) => (Scope::Everything, me.currency.trim().to_string()),
    };

    let months = query.months.unwrap_or(6).clamp(1, 36);

    // En un grupo se mira el gasto total; en el resto de los casos, lo que le
    // tocó pagar al usuario (su parte de cada gasto).
    let (by_category, by_month, totals) = match scope {
        Scope::Group(group_id) => {
            let by_category = sqlx::query_as::<_, CategoryStatRow>(
                "SELECT c.id, c.slug, c.name, c.icon,
                        COALESCE(SUM(e.amount_cents), 0)::bigint AS total_cents,
                        COUNT(e.id)::bigint AS expense_count
                 FROM expenses e JOIN categories c ON c.id = e.category_id
                 WHERE e.group_id = $1 AND e.deleted_at IS NULL
                 GROUP BY c.id, c.slug, c.name, c.icon
                 ORDER BY total_cents DESC",
            )
            .bind(group_id)
            .fetch_all(&state.db)
            .await?;

            let by_month = sqlx::query_as::<_, MonthRow>(
                "SELECT to_char(date_trunc('month', e.expense_date), 'YYYY-MM') AS month,
                        COALESCE(SUM(e.amount_cents), 0)::bigint AS total_cents
                 FROM expenses e
                 WHERE e.group_id = $1 AND e.deleted_at IS NULL
                   AND e.expense_date >= date_trunc('month', CURRENT_DATE)
                                         - make_interval(months => $2)
                 GROUP BY 1 ORDER BY 1",
            )
            .bind(group_id)
            .bind(months)
            .fetch_all(&state.db)
            .await?;

            let totals: (i64, i64) = sqlx::query_as(
                "SELECT COALESCE(SUM(amount_cents), 0)::bigint, COUNT(*)::bigint
                 FROM expenses WHERE group_id = $1 AND deleted_at IS NULL",
            )
            .bind(group_id)
            .fetch_one(&state.db)
            .await?;

            (by_category, by_month, totals)
        }
        Scope::Personal | Scope::Everything => {
            let only_personal = matches!(scope, Scope::Personal);

            let by_category = sqlx::query_as::<_, CategoryStatRow>(
                "SELECT c.id, c.slug, c.name, c.icon,
                        COALESCE(SUM(s.share_cents), 0)::bigint AS total_cents,
                        COUNT(DISTINCT e.id)::bigint AS expense_count
                 FROM expense_splits s
                 JOIN expenses e ON e.id = s.expense_id
                 JOIN categories c ON c.id = e.category_id
                 WHERE s.user_id = $1 AND e.deleted_at IS NULL
                   AND ($2 = FALSE OR e.group_id IS NULL)
                 GROUP BY c.id, c.slug, c.name, c.icon
                 ORDER BY total_cents DESC",
            )
            .bind(user.id)
            .bind(only_personal)
            .fetch_all(&state.db)
            .await?;

            let by_month = sqlx::query_as::<_, MonthRow>(
                "SELECT to_char(date_trunc('month', e.expense_date), 'YYYY-MM') AS month,
                        COALESCE(SUM(s.share_cents), 0)::bigint AS total_cents
                 FROM expense_splits s
                 JOIN expenses e ON e.id = s.expense_id
                 WHERE s.user_id = $1 AND e.deleted_at IS NULL
                   AND ($2 = FALSE OR e.group_id IS NULL)
                   AND e.expense_date >= date_trunc('month', CURRENT_DATE)
                                         - make_interval(months => $3)
                 GROUP BY 1 ORDER BY 1",
            )
            .bind(user.id)
            .bind(only_personal)
            .bind(months)
            .fetch_all(&state.db)
            .await?;

            let totals: (i64, i64) = sqlx::query_as(
                "SELECT COALESCE(SUM(s.share_cents), 0)::bigint, COUNT(DISTINCT e.id)::bigint
                 FROM expense_splits s
                 JOIN expenses e ON e.id = s.expense_id
                 WHERE s.user_id = $1 AND e.deleted_at IS NULL
                   AND ($2 = FALSE OR e.group_id IS NULL)",
            )
            .bind(user.id)
            .bind(only_personal)
            .fetch_one(&state.db)
            .await?;

            (by_category, by_month, totals)
        }
    };

    Ok(Json(StatsResponse {
        currency,
        total_cents: totals.0,
        expense_count: totals.1,
        by_category: by_category
            .into_iter()
            .filter(|c| c.total_cents > 0)
            .map(|c| CategoryBreakdown {
                category: CategoryResponse {
                    id: c.id,
                    slug: c.slug,
                    name: c.name,
                    icon: c.icon,
                },
                total_cents: c.total_cents,
                expense_count: c.expense_count,
            })
            .collect(),
        by_month: by_month
            .into_iter()
            .map(|m| MonthlyPoint {
                month: m.month,
                total_cents: m.total_cents,
            })
            .collect(),
    }))
}
