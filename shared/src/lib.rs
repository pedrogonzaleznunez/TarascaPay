//! DTOs compartidos entre la API y cualquier cliente Rust.
//!
//! Convención de dinero: todos los importes viajan como enteros en centavos
//! (`*_cents`). El formateo a la moneda local es responsabilidad del cliente.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod split;

// ===========================================================================
// Autenticación / usuarios
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    #[serde(default)]
    pub currency: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

/// Vista reducida de un usuario, embebida en otros recursos.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserBrief {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// ===========================================================================
// Categorías
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CategoryResponse {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub icon: String,
}

// ===========================================================================
// Grupos
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateGroupRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub start_date: Option<NaiveDate>,
    #[serde(default)]
    pub end_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateGroupRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub archived: Option<bool>,
    #[serde(default)]
    pub start_date: Option<NaiveDate>,
    #[serde(default)]
    pub end_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub currency: String,
    pub emoji: String,
    pub invite_code: String,
    pub created_by: Uuid,
    pub archived: bool,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    /// Rol del usuario autenticado dentro del grupo.
    pub role: String,
    pub member_count: i64,
    pub expense_count: i64,
    /// Total gastado por el grupo, en centavos.
    pub total_spent_cents: i64,
    /// Saldo del usuario autenticado: positivo = le deben, negativo = debe.
    pub my_balance_cents: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupDetail {
    #[serde(flatten)]
    pub group: GroupResponse,
    pub members: Vec<MemberResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemberResponse {
    pub user: UserBrief,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    /// Saldo neto del miembro dentro del grupo.
    pub balance_cents: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JoinGroupRequest {
    pub invite_code: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddMemberRequest {
    pub email: String,
}

// ===========================================================================
// Gastos
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SplitInput {
    pub user_id: Uuid,
    /// Interpretación según `split_type`:
    /// `exact` → centavos, `percentage` → puntos básicos (10000 = 100%),
    /// `shares` → cantidad de partes. Ignorado en `equal`.
    #[serde(default)]
    pub value: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateExpenseRequest {
    /// `None` ⇒ gasto personal.
    #[serde(default)]
    pub group_id: Option<Uuid>,
    pub description: String,
    #[serde(default)]
    pub notes: Option<String>,
    pub amount_cents: i64,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub category_id: Option<i32>,
    #[serde(default)]
    pub expense_date: Option<NaiveDate>,
    /// `None` ⇒ lo pagó el usuario autenticado.
    #[serde(default)]
    pub paid_by: Option<Uuid>,
    #[serde(default = "default_split_type")]
    pub split_type: String,
    /// Participantes. Si viene vacío en un gasto de grupo se reparte
    /// en partes iguales entre todos los miembros.
    #[serde(default)]
    pub splits: Vec<SplitInput>,
    /// Ids de adjuntos ya subidos que se vinculan a este gasto.
    #[serde(default)]
    pub attachment_ids: Vec<Uuid>,
}

fn default_split_type() -> String {
    "equal".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateExpenseRequest {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub amount_cents: Option<i64>,
    #[serde(default)]
    pub category_id: Option<i32>,
    #[serde(default)]
    pub expense_date: Option<NaiveDate>,
    #[serde(default)]
    pub paid_by: Option<Uuid>,
    #[serde(default)]
    pub split_type: Option<String>,
    #[serde(default)]
    pub splits: Option<Vec<SplitInput>>,
    #[serde(default)]
    pub attachment_ids: Option<Vec<Uuid>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExpenseSplitResponse {
    pub user: UserBrief,
    pub share_cents: i64,
    pub split_value: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttachmentResponse {
    pub id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExpenseResponse {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub group_name: Option<String>,
    pub description: String,
    pub notes: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub category: CategoryResponse,
    pub expense_date: NaiveDate,
    pub split_type: String,
    pub paid_by: UserBrief,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub splits: Vec<ExpenseSplitResponse>,
    pub attachments: Vec<AttachmentResponse>,
    pub comment_count: i64,
    /// Impacto del gasto sobre el usuario autenticado.
    /// Positivo = puso de más (le deben), negativo = le toca pagar.
    pub my_net_cents: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ExpenseQuery {
    #[serde(default)]
    pub group_id: Option<Uuid>,
    #[serde(default)]
    pub personal: Option<bool>,
    #[serde(default)]
    pub category_id: Option<i32>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub from: Option<NaiveDate>,
    #[serde(default)]
    pub to: Option<NaiveDate>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

// ===========================================================================
// Balances y liquidaciones
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BalanceEntry {
    pub user: UserBrief,
    /// Positivo = le deben. Negativo = debe.
    pub net_cents: i64,
    pub paid_cents: i64,
    pub owed_cents: i64,
}

/// Transferencia sugerida para saldar cuentas con el mínimo de movimientos.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SuggestedTransfer {
    pub from: UserBrief,
    pub to: UserBrief,
    pub amount_cents: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupBalances {
    pub currency: String,
    pub total_spent_cents: i64,
    pub balances: Vec<BalanceEntry>,
    pub transfers: Vec<SuggestedTransfer>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateSettlementRequest {
    pub from_user: Uuid,
    pub to_user: Uuid,
    pub amount_cents: i64,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub settled_at: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SettlementResponse {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub from: UserBrief,
    pub to: UserBrief,
    pub amount_cents: i64,
    pub currency: String,
    pub note: Option<String>,
    pub settled_at: NaiveDate,
    pub created_at: DateTime<Utc>,
}

// ===========================================================================
// Comentarios y actividad
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateCommentRequest {
    pub body: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommentResponse {
    pub id: Uuid,
    pub user: UserBrief,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActivityResponse {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub actor: UserBrief,
    pub kind: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ===========================================================================
// Panel / estadísticas
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CategoryBreakdown {
    pub category: CategoryResponse,
    pub total_cents: i64,
    pub expense_count: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MonthlyPoint {
    /// Formato `YYYY-MM`.
    pub month: String,
    pub total_cents: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DebtSummary {
    pub user: UserBrief,
    /// Positivo = esa persona te debe. Negativo = vos le debés.
    pub net_cents: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardSummary {
    pub currency: String,
    /// Suma de saldos positivos: total que te deben.
    pub you_are_owed_cents: i64,
    /// Suma de saldos negativos (en positivo): total que debés.
    pub you_owe_cents: i64,
    pub net_cents: i64,
    pub personal_this_month_cents: i64,
    pub group_this_month_cents: i64,
    pub active_groups: i64,
    pub debts: Vec<DebtSummary>,
    pub recent_expenses: Vec<ExpenseResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatsResponse {
    pub currency: String,
    pub total_cents: i64,
    pub expense_count: i64,
    pub by_category: Vec<CategoryBreakdown>,
    pub by_month: Vec<MonthlyPoint>,
}

// ===========================================================================
// Utilidades de respuesta
// ===========================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorResponse {
    pub error: String,
}
