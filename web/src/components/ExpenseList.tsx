// Listado de gastos y su vista de detalle.

import { useEffect, useState } from 'react'
import { MessageSquare, Paperclip, Pencil, Receipt, Trash2 } from 'lucide-react'

import { api } from '../lib/api'
import { useAuth } from '../lib/auth'
import { formatDate, money, relativeTime } from '../lib/format'
import { emitRefresh, errorMessage } from '../lib/hooks'
import { useToast } from '../lib/toast'
import type { Comment, Expense } from '../lib/types'
import { ExpenseFormModal } from './ExpenseForm'
import {
  Avatar,
  Badge,
  Button,
  ConfirmDialog,
  EmptyState,
  Input,
  Modal,
  SignedAmount,
  Spinner,
  cn,
} from './ui'

export function ExpenseList({
  expenses,
  loading,
  emptyTitle = 'Todavía no hay gastos',
  emptyDescription,
  emptyAction,
  showGroup = false,
}: {
  expenses: Expense[]
  loading?: boolean
  emptyTitle?: string
  emptyDescription?: string
  emptyAction?: React.ReactNode
  showGroup?: boolean
}) {
  const [selected, setSelected] = useState<Expense | null>(null)

  if (loading) return <Spinner />

  if (expenses.length === 0) {
    return (
      <EmptyState
        icon={<Receipt className="size-10" />}
        title={emptyTitle}
        description={emptyDescription}
        action={emptyAction}
      />
    )
  }

  return (
    <>
      <ul className="divide-y divide-slate-200 overflow-hidden rounded-2xl border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900">
        {expenses.map((expense) => (
          <li key={expense.id}>
            <button
              type="button"
              onClick={() => setSelected(expense)}
              className="flex w-full items-center gap-3 px-4 py-3 text-left transition hover:bg-slate-50 dark:hover:bg-slate-800/60"
            >
              <span
                aria-hidden
                className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-slate-100 text-lg dark:bg-slate-800"
              >
                {expense.category.icon}
              </span>

              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{expense.description}</p>
                <p className="truncate text-xs text-slate-500 dark:text-slate-400">
                  {formatDate(expense.expense_date)}
                  {' · '}
                  {expense.group_id
                    ? `Pagó ${expense.paid_by.display_name}`
                    : expense.category.name}
                  {showGroup && expense.group_name ? ` · ${expense.group_name}` : ''}
                </p>
              </div>

              <div className="flex shrink-0 items-center gap-2">
                {expense.attachments.length > 0 && (
                  <Paperclip className="size-3.5 text-slate-400" aria-label="Tiene comprobante" />
                )}
                {expense.comment_count > 0 && (
                  <span className="flex items-center gap-0.5 text-xs text-slate-400">
                    <MessageSquare className="size-3.5" />
                    {expense.comment_count}
                  </span>
                )}
                <div className="text-right">
                  <p className="tabular font-semibold">
                    {money(expense.amount_cents, expense.currency)}
                  </p>
                  {expense.group_id && expense.my_net_cents !== 0 && (
                    <SignedAmount
                      cents={expense.my_net_cents}
                      currency={expense.currency}
                      className="text-xs"
                    />
                  )}
                </div>
              </div>
            </button>
          </li>
        ))}
      </ul>

      <ExpenseDetailModal
        expense={selected}
        onClose={() => setSelected(null)}
      />
    </>
  )
}

// ---------------------------------------------------------------------------
// Detalle
// ---------------------------------------------------------------------------

export function ExpenseDetailModal({
  expense,
  onClose,
}: {
  expense: Expense | null
  onClose: () => void
}) {
  const { user } = useAuth()
  const toast = useToast()

  const [comments, setComments] = useState<Comment[]>([])
  const [newComment, setNewComment] = useState('')
  const [sending, setSending] = useState(false)
  const [editing, setEditing] = useState(false)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [preview, setPreview] = useState<string | null>(null)

  useEffect(() => {
    if (!expense) {
      setComments([])
      return
    }
    let cancelled = false
    api
      .comments(expense.id)
      .then((result) => !cancelled && setComments(result))
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [expense])

  if (!expense) return null

  const canModify =
    expense.created_by === user?.id || expense.paid_by.id === user?.id

  async function handleComment(event: React.FormEvent) {
    event.preventDefault()
    if (!expense || !newComment.trim()) return
    setSending(true)
    try {
      const created = await api.addComment(expense.id, newComment.trim())
      setComments((current) => [...current, created])
      setNewComment('')
    } catch (err) {
      toast.error(errorMessage(err))
    } finally {
      setSending(false)
    }
  }

  async function handleDelete() {
    if (!expense) return
    setDeleting(true)
    try {
      await api.deleteExpense(expense.id)
      toast.success('Gasto eliminado')
      emitRefresh()
      setConfirmDelete(false)
      onClose()
    } catch (err) {
      toast.error(errorMessage(err))
    } finally {
      setDeleting(false)
    }
  }

  return (
    <>
      <Modal open={!editing} onClose={onClose} title={expense.description}>
        <div className="space-y-5">
          <div className="flex items-start justify-between gap-4">
            <div>
              <p className="tabular text-3xl font-bold">
                {money(expense.amount_cents, expense.currency)}
              </p>
              <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
                {expense.category.icon} {expense.category.name} · {formatDate(expense.expense_date)}
              </p>
            </div>
            {canModify && (
              <div className="flex gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  icon={<Pencil className="size-4" />}
                  onClick={() => setEditing(true)}
                  aria-label="Editar"
                />
                <Button
                  size="sm"
                  variant="ghost"
                  icon={<Trash2 className="size-4" />}
                  onClick={() => setConfirmDelete(true)}
                  aria-label="Eliminar"
                  className="text-rose-600 hover:bg-rose-50 dark:hover:bg-rose-950/50"
                />
              </div>
            )}
          </div>

          {expense.group_id && (
            <div className="flex items-center gap-2 rounded-xl bg-slate-50 px-3 py-2.5 text-sm dark:bg-slate-800/60">
              <Avatar user={expense.paid_by} size="sm" />
              <span>
                Pagó <strong>{expense.paid_by.id === user?.id ? 'yo' : expense.paid_by.display_name}</strong>
              </span>
              {expense.group_name && <Badge>{expense.group_name}</Badge>}
            </div>
          )}

          {expense.notes && (
            <p className="rounded-xl bg-amber-50 px-3 py-2.5 text-sm text-amber-900 dark:bg-amber-950/40 dark:text-amber-200">
              {expense.notes}
            </p>
          )}

          {expense.group_id && expense.splits.length > 0 && (
            <div>
              <h3 className="mb-2 text-sm font-semibold">Reparto</h3>
              <ul className="divide-y divide-slate-200 overflow-hidden rounded-xl border border-slate-200 dark:divide-slate-800 dark:border-slate-800">
                {expense.splits.map((split) => (
                  <li
                    key={split.user.id}
                    className="flex items-center gap-3 px-3 py-2 text-sm"
                  >
                    <Avatar user={split.user} size="xs" />
                    <span className="flex-1 truncate">
                      {split.user.id === user?.id ? 'Yo' : split.user.display_name}
                    </span>
                    <span className="tabular font-medium">
                      {money(split.share_cents, expense.currency)}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {expense.attachments.length > 0 && (
            <div>
              <h3 className="mb-2 text-sm font-semibold">Comprobantes</h3>
              <div className="flex flex-wrap gap-2">
                {expense.attachments.map((attachment) => (
                  <button
                    key={attachment.id}
                    type="button"
                    onClick={() => setPreview(api.fileUrl(attachment.id))}
                    className="size-24 overflow-hidden rounded-xl border border-slate-200 transition hover:opacity-80 dark:border-slate-700"
                  >
                    {attachment.mime_type.startsWith('image/') ? (
                      <img
                        src={api.fileUrl(attachment.id)}
                        alt={attachment.file_name}
                        className="size-full object-cover"
                      />
                    ) : (
                      <span className="flex size-full items-center justify-center bg-slate-100 text-xs dark:bg-slate-800">
                        PDF
                      </span>
                    )}
                  </button>
                ))}
              </div>
            </div>
          )}

          <div>
            <h3 className="mb-2 text-sm font-semibold">Comentarios</h3>
            {comments.length > 0 && (
              <ul className="mb-3 space-y-2">
                {comments.map((comment) => (
                  <li key={comment.id} className="flex gap-2">
                    <Avatar user={comment.user} size="xs" className="mt-0.5" />
                    <div className="min-w-0 flex-1 rounded-xl bg-slate-100 px-3 py-2 dark:bg-slate-800">
                      <p className="text-xs font-medium">
                        {comment.user.display_name}
                        <span className="ml-2 font-normal text-slate-500">
                          {relativeTime(comment.created_at)}
                        </span>
                      </p>
                      <p className="mt-0.5 text-sm break-words">{comment.body}</p>
                    </div>
                  </li>
                ))}
              </ul>
            )}
            <form onSubmit={handleComment} className="flex gap-2">
              <Input
                value={newComment}
                onChange={(e) => setNewComment(e.target.value)}
                placeholder="Escribí un comentario…"
                maxLength={2000}
              />
              <Button type="submit" loading={sending} disabled={!newComment.trim()}>
                Enviar
              </Button>
            </form>
          </div>
        </div>
      </Modal>

      <ExpenseFormModal
        open={editing}
        expense={expense}
        onClose={() => setEditing(false)}
        onSaved={() => {
          setEditing(false)
          onClose()
        }}
      />

      <ConfirmDialog
        open={confirmDelete}
        title="Eliminar gasto"
        message={`¿Seguro que querés eliminar "${expense.description}"? Los saldos del grupo se recalculan.`}
        confirmLabel="Eliminar"
        danger
        loading={deleting}
        onConfirm={handleDelete}
        onCancel={() => setConfirmDelete(false)}
      />

      {/* Comprobante a pantalla completa. */}
      {preview && (
        <div
          className="fixed inset-0 z-60 flex items-center justify-center bg-slate-950/90 p-4"
          onClick={() => setPreview(null)}
        >
          <img
            src={preview}
            alt="Comprobante"
            className={cn('max-h-full max-w-full rounded-xl object-contain')}
          />
        </div>
      )}
    </>
  )
}
