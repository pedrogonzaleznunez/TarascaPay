//! Filas tal como vienen de la base de datos y su conversión a DTOs.

use chrono::{DateTime, NaiveDate, Utc};
use shared::{
    AttachmentResponse, CategoryResponse, CommentResponse, SettlementResponse, UserBrief,
    UserResponse,
};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

impl UserRow {
    pub fn into_response(self) -> UserResponse {
        UserResponse {
            id: self.id,
            email: self.email,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            currency: self.currency.trim().to_string(),
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct UserBriefRow {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

impl From<UserBriefRow> for UserBrief {
    fn from(row: UserBriefRow) -> Self {
        UserBrief {
            id: row.id,
            display_name: row.display_name,
            email: row.email,
            avatar_url: row.avatar_url,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct CategoryRow {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub icon: String,
}

impl From<CategoryRow> for CategoryResponse {
    fn from(row: CategoryRow) -> Self {
        CategoryResponse {
            id: row.id,
            slug: row.slug,
            name: row.name,
            icon: row.icon,
        }
    }
}

/// Fila plana del listado de gastos: incluye datos del pagador, la categoría
/// y el grupo para evitar una consulta por gasto.
#[derive(Debug, Clone, FromRow)]
pub struct ExpenseRow {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub group_name: Option<String>,
    pub description: String,
    pub notes: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub expense_date: NaiveDate,
    pub split_type: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub category_id: i32,
    pub category_slug: String,
    pub category_name: String,
    pub category_icon: String,
    pub payer_id: Uuid,
    pub payer_name: String,
    pub payer_email: String,
    pub payer_avatar: Option<String>,
}

impl ExpenseRow {
    pub fn category(&self) -> CategoryResponse {
        CategoryResponse {
            id: self.category_id,
            slug: self.category_slug.clone(),
            name: self.category_name.clone(),
            icon: self.category_icon.clone(),
        }
    }

    pub fn payer(&self) -> UserBrief {
        UserBrief {
            id: self.payer_id,
            display_name: self.payer_name.clone(),
            email: self.payer_email.clone(),
            avatar_url: self.payer_avatar.clone(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct SplitRow {
    pub expense_id: Uuid,
    pub share_cents: i64,
    pub split_value: i64,
    pub user_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AttachmentRow {
    pub id: Uuid,
    pub expense_id: Option<Uuid>,
    pub uploaded_by: Uuid,
    pub file_name: String,
    pub stored_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}

impl AttachmentRow {
    pub fn into_response(self) -> AttachmentResponse {
        AttachmentResponse {
            url: format!("/api/attachments/{}/file", self.id),
            id: self.id,
            file_name: self.file_name,
            mime_type: self.mime_type,
            size_bytes: self.size_bytes,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct SettlementRow {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub amount_cents: i64,
    pub currency: String,
    pub note: Option<String>,
    pub settled_at: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub from_id: Uuid,
    pub from_name: String,
    pub from_email: String,
    pub from_avatar: Option<String>,
    pub to_id: Uuid,
    pub to_name: String,
    pub to_email: String,
    pub to_avatar: Option<String>,
}

impl From<SettlementRow> for SettlementResponse {
    fn from(row: SettlementRow) -> Self {
        SettlementResponse {
            id: row.id,
            group_id: row.group_id,
            from: UserBrief {
                id: row.from_id,
                display_name: row.from_name,
                email: row.from_email,
                avatar_url: row.from_avatar,
            },
            to: UserBrief {
                id: row.to_id,
                display_name: row.to_name,
                email: row.to_email,
                avatar_url: row.to_avatar,
            },
            amount_cents: row.amount_cents,
            currency: row.currency.trim().to_string(),
            note: row.note,
            settled_at: row.settled_at,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct CommentRow {
    pub id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

impl From<CommentRow> for CommentResponse {
    fn from(row: CommentRow) -> Self {
        CommentResponse {
            id: row.id,
            user: UserBrief {
                id: row.user_id,
                display_name: row.display_name,
                email: row.email,
                avatar_url: row.avatar_url,
            },
            body: row.body,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ActivityRow {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub kind: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub actor_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

/// Agregado de saldos por miembro dentro de un grupo.
#[derive(Debug, Clone, FromRow)]
pub struct BalanceRow {
    pub user_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub paid_cents: i64,
    pub owed_cents: i64,
}

impl BalanceRow {
    pub fn net_cents(&self) -> i64 {
        self.paid_cents - self.owed_cents
    }

    pub fn user(&self) -> UserBrief {
        UserBrief {
            id: self.user_id,
            display_name: self.display_name.clone(),
            email: self.email.clone(),
            avatar_url: self.avatar_url.clone(),
        }
    }
}
