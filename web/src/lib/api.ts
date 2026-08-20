// Cliente HTTP tipado contra la API de TarascaPay.

import type {
  Activity,
  Attachment,
  AuthResponse,
  Category,
  Comment,
  CreateExpenseBody,
  DashboardSummary,
  Expense,
  Group,
  GroupBalances,
  GroupDetail,
  GroupKind,
  Member,
  Settlement,
  Stats,
  User,
} from './types'

const BASE = '/api'
const TOKEN_KEY = 'tarascapay.token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string | null) {
  if (token) localStorage.setItem(TOKEN_KEY, token)
  else localStorage.removeItem(TOKEN_KEY)
}

/** Error con el mensaje que devolvió la API, listo para mostrar al usuario. */
export class ApiError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

/** Se dispara cuando la sesión deja de ser válida, para que la app deslogue. */
export const onUnauthorized = { handler: null as null | (() => void) }

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = getToken()
  const headers = new Headers(init.headers)

  if (token) headers.set('Authorization', `Bearer ${token}`)
  if (init.body && !(init.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json')
  }

  let response: Response
  try {
    response = await fetch(`${BASE}${path}`, { ...init, headers })
  } catch {
    throw new ApiError('No se pudo conectar con el servidor. ¿Está corriendo la API?', 0)
  }

  if (response.status === 401) {
    // Token vencido o inválido: cerramos sesión en vez de dejar la app a medias.
    setToken(null)
    onUnauthorized.handler?.()
    throw new ApiError('Tu sesión expiró. Volvé a iniciar sesión.', 401)
  }

  if (!response.ok) {
    const message = await response
      .json()
      .then((body) => (body as { error?: string }).error)
      .catch(() => null)
    throw new ApiError(message ?? `Error ${response.status}`, response.status)
  }

  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

const get = <T,>(path: string) => request<T>(path)
const post = <T,>(path: string, body?: unknown) =>
  request<T>(path, { method: 'POST', body: body === undefined ? undefined : JSON.stringify(body) })
const patch = <T,>(path: string, body: unknown) =>
  request<T>(path, { method: 'PATCH', body: JSON.stringify(body) })
const put = <T,>(path: string, body: unknown) =>
  request<T>(path, { method: 'PUT', body: JSON.stringify(body) })
const del = <T,>(path: string) => request<T>(path, { method: 'DELETE' })

function query(params: Record<string, string | number | boolean | undefined | null>) {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') search.set(key, String(value))
  }
  const qs = search.toString()
  return qs ? `?${qs}` : ''
}

export const api = {
  // --- Autenticación ---
  register: (body: {
    email: string
    password: string
    display_name: string
    currency?: string
  }) => post<AuthResponse>('/auth/register', body),

  login: (body: { email: string; password: string }) => post<AuthResponse>('/auth/login', body),

  me: () => get<User>('/auth/me'),

  updateProfile: (body: { display_name?: string; currency?: string; avatar_url?: string }) =>
    put<User>('/auth/profile', body),

  changePassword: (body: { current_password: string; new_password: string }) =>
    put<{ message: string }>('/auth/password', body),

  // --- Grupos ---
  groups: () => get<Group[]>('/groups'),

  group: (id: string) => get<GroupDetail>(`/groups/${id}`),

  createGroup: (body: {
    name: string
    description?: string
    kind?: GroupKind
    currency?: string
    emoji?: string
    start_date?: string | null
    end_date?: string | null
  }) => post<GroupDetail>('/groups', body),

  updateGroup: (
    id: string,
    body: {
      name?: string
      description?: string
      emoji?: string
      kind?: GroupKind
      archived?: boolean
      start_date?: string | null
      end_date?: string | null
    },
  ) => patch<GroupDetail>(`/groups/${id}`, body),

  deleteGroup: (id: string) => del<{ message: string }>(`/groups/${id}`),

  joinGroup: (invite_code: string) => post<GroupDetail>('/groups/join', { invite_code }),

  addMember: (groupId: string, email: string) =>
    post<Member[]>(`/groups/${groupId}/members`, { email }),

  removeMember: (groupId: string, userId: string) =>
    del<Member[]>(`/groups/${groupId}/members/${userId}`),

  regenerateInvite: (groupId: string) =>
    post<{ message: string }>(`/groups/${groupId}/invite-code`),

  balances: (groupId: string) => get<GroupBalances>(`/groups/${groupId}/balances`),

  activity: (groupId: string) => get<Activity[]>(`/groups/${groupId}/activity`),

  // --- Gastos ---
  categories: () => get<Category[]>('/categories'),

  expenses: (params: {
    group_id?: string
    personal?: boolean
    category_id?: number
    search?: string
    from?: string
    to?: string
    limit?: number
    offset?: number
  }) => get<Expense[]>(`/expenses${query(params)}`),

  expense: (id: string) => get<Expense>(`/expenses/${id}`),

  createExpense: (body: CreateExpenseBody) => post<Expense>('/expenses', body),

  updateExpense: (id: string, body: Partial<CreateExpenseBody>) =>
    patch<Expense>(`/expenses/${id}`, body),

  deleteExpense: (id: string) => del<{ message: string }>(`/expenses/${id}`),

  comments: (expenseId: string) => get<Comment[]>(`/expenses/${expenseId}/comments`),

  addComment: (expenseId: string, body: string) =>
    post<Comment>(`/expenses/${expenseId}/comments`, { body }),

  // --- Liquidaciones ---
  settlements: (groupId: string) => get<Settlement[]>(`/groups/${groupId}/settlements`),

  createSettlement: (
    groupId: string,
    body: {
      from_user: string
      to_user: string
      amount_cents: number
      note?: string
      settled_at?: string
    },
  ) => post<Settlement>(`/groups/${groupId}/settlements`, body),

  deleteSettlement: (id: string) => del<{ message: string }>(`/settlements/${id}`),

  // --- Comprobantes ---
  upload: (file: File) => {
    const form = new FormData()
    form.append('file', file)
    return request<Attachment>('/attachments', { method: 'POST', body: form })
  },

  deleteAttachment: (id: string) => del<{ message: string }>(`/attachments/${id}`),

  /**
   * URL directa de un comprobante. Lleva el token en la query porque una
   * etiqueta <img> no puede mandar cabeceras.
   */
  fileUrl: (attachmentId: string) =>
    `${BASE}/attachments/${attachmentId}/file?token=${encodeURIComponent(getToken() ?? '')}`,

  // --- Panel ---
  dashboard: () => get<DashboardSummary>('/dashboard'),

  stats: (params: { group_id?: string; personal?: boolean; months?: number }) =>
    get<Stats>(`/stats${query(params)}`),
}
