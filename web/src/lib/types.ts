// Espejo en TypeScript de los DTOs de la API (`shared/src/lib.rs`).
// Todos los importes son enteros en centavos.

export type SplitType = 'equal' | 'exact' | 'percentage' | 'shares'
export type GroupKind = 'trip' | 'home' | 'couple' | 'event' | 'other'

export interface User {
  id: string
  email: string
  display_name: string
  avatar_url: string | null
  currency: string
  created_at: string
}

export interface UserBrief {
  id: string
  display_name: string
  email: string
  avatar_url: string | null
}

export interface AuthResponse {
  token: string
  user: User
}

export interface Category {
  id: number
  slug: string
  name: string
  icon: string
}

export interface Group {
  id: string
  name: string
  description: string | null
  kind: GroupKind
  currency: string
  emoji: string
  invite_code: string
  created_by: string
  archived: boolean
  start_date: string | null
  end_date: string | null
  created_at: string
  role: 'owner' | 'member'
  member_count: number
  expense_count: number
  total_spent_cents: number
  my_balance_cents: number
}

export interface Member {
  user: UserBrief
  role: 'owner' | 'member'
  joined_at: string
  balance_cents: number
}

/** `GroupDetail` llega aplanado: el grupo y sus miembros en el mismo objeto. */
export type GroupDetail = Group & { members: Member[] }

export interface ExpenseSplit {
  user: UserBrief
  share_cents: number
  split_value: number
}

export interface Attachment {
  id: string
  file_name: string
  mime_type: string
  size_bytes: number
  url: string
  created_at: string
}

export interface Expense {
  id: string
  group_id: string | null
  group_name: string | null
  description: string
  notes: string | null
  amount_cents: number
  currency: string
  category: Category
  expense_date: string
  split_type: SplitType
  paid_by: UserBrief
  created_by: string
  created_at: string
  splits: ExpenseSplit[]
  attachments: Attachment[]
  comment_count: number
  my_net_cents: number
}

export interface SplitInput {
  user_id: string
  value: number
}

export interface CreateExpenseBody {
  group_id?: string | null
  description: string
  notes?: string | null
  amount_cents: number
  currency?: string
  category_id?: number
  expense_date?: string
  paid_by?: string
  split_type: SplitType
  splits: SplitInput[]
  attachment_ids: string[]
}

export interface BalanceEntry {
  user: UserBrief
  net_cents: number
  paid_cents: number
  owed_cents: number
}

export interface SuggestedTransfer {
  from: UserBrief
  to: UserBrief
  amount_cents: number
}

export interface GroupBalances {
  currency: string
  total_spent_cents: number
  balances: BalanceEntry[]
  transfers: SuggestedTransfer[]
}

export interface Settlement {
  id: string
  group_id: string | null
  from: UserBrief
  to: UserBrief
  amount_cents: number
  currency: string
  note: string | null
  settled_at: string
  created_at: string
}

export interface Comment {
  id: string
  user: UserBrief
  body: string
  created_at: string
}

export interface Activity {
  id: string
  group_id: string | null
  actor: UserBrief
  kind: string
  payload: Record<string, unknown>
  created_at: string
}

export interface DebtSummary {
  user: UserBrief
  net_cents: number
}

export interface DashboardSummary {
  currency: string
  you_are_owed_cents: number
  you_owe_cents: number
  net_cents: number
  personal_this_month_cents: number
  group_this_month_cents: number
  active_groups: number
  debts: DebtSummary[]
  recent_expenses: Expense[]
}

export interface CategoryBreakdown {
  category: Category
  total_cents: number
  expense_count: number
}

export interface MonthlyPoint {
  month: string
  total_cents: number
}

export interface Stats {
  currency: string
  total_cents: number
  expense_count: number
  by_category: CategoryBreakdown[]
  by_month: MonthlyPoint[]
}
