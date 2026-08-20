// Detalle de un grupo: gastos, balances, liquidaciones, miembros y actividad.

import { useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'
import {
  ArrowLeft,
  Check,
  Copy,
  HandCoins,
  Plus,
  Scale,
  Settings,
  Trash2,
  UserPlus,
} from 'lucide-react'

import { api } from '../lib/api'
import { useAuth } from '../lib/auth'
import { formatDate, money, relativeTime } from '../lib/format'
import { emitRefresh, errorMessage, useAsync } from '../lib/hooks'
import { useToast } from '../lib/toast'
import type { GroupBalances, GroupDetail, Member, SuggestedTransfer } from '../lib/types'
import { ExpenseFormModal } from '../components/ExpenseForm'
import { ExpenseList } from '../components/ExpenseList'
import {
  Avatar,
  Badge,
  Button,
  Card,
  ConfirmDialog,
  EmptyState,
  ErrorMessage,
  Field,
  Input,
  Modal,
  SignedAmount,
  Spinner,
  Textarea,
  cn,
} from '../components/ui'

type Tab = 'gastos' | 'balances' | 'miembros' | 'actividad'

const TABS: { id: Tab; label: string }[] = [
  { id: 'gastos', label: 'Gastos' },
  { id: 'balances', label: 'Balances' },
  { id: 'miembros', label: 'Miembros' },
  { id: 'actividad', label: 'Actividad' },
]

export function GroupDetailPage() {
  const { groupId = '' } = useParams()
  const [tab, setTab] = useState<Tab>('gastos')
  const [addingExpense, setAddingExpense] = useState(false)

  const group = useAsync(() => api.group(groupId), [groupId])
  const balances = useAsync(() => api.balances(groupId), [groupId])
  const expenses = useAsync(() => api.expenses({ group_id: groupId, limit: 200 }), [groupId])

  if (group.loading && !group.data) return <Spinner />
  if (group.error) return <ErrorMessage message={group.error} />
  if (!group.data) return null

  const detail = group.data

  function refreshAll() {
    group.reload()
    balances.reload()
    expenses.reload()
  }

  return (
    <>
      <Link
        to="/grupos"
        className="mb-4 inline-flex items-center gap-1 text-sm text-slate-500 hover:text-slate-700 dark:hover:text-slate-300"
      >
        <ArrowLeft className="size-4" /> Grupos
      </Link>

      <GroupHeader detail={detail} onChanged={refreshAll} />

      {/* Pestañas */}
      <div className="mb-5 flex gap-1 overflow-x-auto border-b border-slate-200 dark:border-slate-800">
        {TABS.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => setTab(item.id)}
            className={cn(
              '-mb-px border-b-2 px-4 py-2.5 text-sm font-medium whitespace-nowrap transition',
              tab === item.id
                ? 'border-brand-600 text-brand-600 dark:text-brand-400'
                : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300',
            )}
          >
            {item.label}
          </button>
        ))}
      </div>

      {tab === 'gastos' && (
        <>
          <div className="mb-3 flex justify-end">
            <Button
              size="sm"
              icon={<Plus className="size-4" />}
              onClick={() => setAddingExpense(true)}
            >
              Agregar gasto
            </Button>
          </div>
          <ExpenseList
            expenses={expenses.data ?? []}
            loading={expenses.loading && !expenses.data}
            emptyTitle="Sin gastos todavía"
            emptyDescription="Cargá el primer gasto del grupo y se reparte automáticamente."
            emptyAction={<Button onClick={() => setAddingExpense(true)}>Agregar gasto</Button>}
          />
        </>
      )}

      {tab === 'balances' && (
        <BalancesTab
          groupId={groupId}
          balances={balances.data}
          loading={balances.loading && !balances.data}
          error={balances.error}
          onSettled={refreshAll}
        />
      )}

      {tab === 'miembros' && <MembersTab detail={detail} onChanged={refreshAll} />}

      {tab === 'actividad' && <ActivityTab groupId={groupId} />}

      <ExpenseFormModal
        open={addingExpense}
        groupId={groupId}
        onClose={() => setAddingExpense(false)}
        onSaved={() => {
          setAddingExpense(false)
          refreshAll()
        }}
      />
    </>
  )
}

// ---------------------------------------------------------------------------
// Encabezado
// ---------------------------------------------------------------------------

function GroupHeader({ detail, onChanged }: { detail: GroupDetail; onChanged: () => void }) {
  const toast = useToast()
  const [copied, setCopied] = useState(false)
  const [settings, setSettings] = useState(false)

  async function copyCode() {
    try {
      await navigator.clipboard.writeText(detail.invite_code)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast.info(`Código: ${detail.invite_code}`)
    }
  }

  return (
    <>
      <Card className="mb-5 p-5">
        <div className="flex items-start gap-4">
          <span aria-hidden className="text-4xl">
            {detail.emoji}
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="truncate text-2xl font-bold tracking-tight">{detail.name}</h1>
              {detail.archived && <Badge>Archivado</Badge>}
            </div>
            {detail.description && (
              <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
                {detail.description}
              </p>
            )}
            {detail.start_date && (
              <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                {formatDate(detail.start_date)}
                {detail.end_date ? ` — ${formatDate(detail.end_date)}` : ''}
              </p>
            )}
          </div>
          {detail.role === 'owner' && (
            <Button
              variant="ghost"
              size="sm"
              icon={<Settings className="size-4" />}
              aria-label="Ajustes del grupo"
              onClick={() => setSettings(true)}
            />
          )}
        </div>

        <div className="mt-5 flex flex-wrap items-end justify-between gap-4">
          <div>
            <p className="text-xs text-slate-500 dark:text-slate-400">Total del grupo</p>
            <p className="tabular text-xl font-bold">
              {money(detail.total_spent_cents, detail.currency)}
            </p>
          </div>
          <div>
            <p className="text-xs text-slate-500 dark:text-slate-400">Tu saldo</p>
            <SignedAmount
              cents={detail.my_balance_cents}
              currency={detail.currency}
              className="text-xl"
            />
          </div>
          <button
            type="button"
            onClick={copyCode}
            className="flex items-center gap-2 rounded-xl bg-slate-100 px-3 py-2 text-sm font-medium transition hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700"
          >
            {copied ? <Check className="size-4 text-emerald-500" /> : <Copy className="size-4" />}
            <span className="tabular tracking-wider">{detail.invite_code}</span>
          </button>
        </div>
      </Card>

      <GroupSettingsModal
        open={settings}
        detail={detail}
        onClose={() => setSettings(false)}
        onChanged={onChanged}
      />
    </>
  )
}

function GroupSettingsModal({
  open,
  detail,
  onClose,
  onChanged,
}: {
  open: boolean
  detail: GroupDetail
  onClose: () => void
  onChanged: () => void
}) {
  const toast = useToast()
  const navigate = useNavigate()
  const [name, setName] = useState(detail.name)
  const [description, setDescription] = useState(detail.description ?? '')
  const [saving, setSaving] = useState(false)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function save(patch: Parameters<typeof api.updateGroup>[1]) {
    setSaving(true)
    setError(null)
    try {
      await api.updateGroup(detail.id, patch)
      toast.success('Grupo actualizado')
      onChanged()
      onClose()
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  async function handleDelete() {
    setDeleting(true)
    try {
      await api.deleteGroup(detail.id)
      toast.success('Grupo eliminado')
      navigate('/grupos')
    } catch (err) {
      toast.error(errorMessage(err))
    } finally {
      setDeleting(false)
    }
  }

  return (
    <>
      <Modal open={open} onClose={onClose} title="Ajustes del grupo">
        <div className="space-y-4">
          {error && <ErrorMessage message={error} />}

          <Field label="Nombre">
            <Input value={name} onChange={(e) => setName(e.target.value)} maxLength={80} />
          </Field>

          <Field label="Descripción">
            <Textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
            />
          </Field>

          <Button
            loading={saving}
            onClick={() => save({ name, description })}
            className="w-full"
          >
            Guardar cambios
          </Button>

          <div className="border-t border-slate-200 pt-4 dark:border-slate-800">
            <Button
              variant="secondary"
              className="w-full"
              onClick={() => save({ archived: !detail.archived })}
            >
              {detail.archived ? 'Desarchivar grupo' : 'Archivar grupo'}
            </Button>
            <p className="mt-1.5 text-xs text-slate-500 dark:text-slate-400">
              Un grupo archivado deja de contarse en el panel, pero conserva el historial.
            </p>
          </div>

          <div className="border-t border-slate-200 pt-4 dark:border-slate-800">
            <Button
              variant="danger"
              className="w-full"
              icon={<Trash2 className="size-4" />}
              onClick={() => setConfirmDelete(true)}
            >
              Eliminar grupo
            </Button>
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={confirmDelete}
        title="Eliminar grupo"
        message={`Se van a borrar todos los gastos, comprobantes y saldos de "${detail.name}". Esta acción no se puede deshacer.`}
        confirmLabel="Eliminar todo"
        danger
        loading={deleting}
        onConfirm={handleDelete}
        onCancel={() => setConfirmDelete(false)}
      />
    </>
  )
}

// ---------------------------------------------------------------------------
// Balances
// ---------------------------------------------------------------------------

function BalancesTab({
  groupId,
  balances,
  loading,
  error,
  onSettled,
}: {
  groupId: string
  balances: GroupBalances | null
  loading: boolean
  error: string | null
  onSettled: () => void
}) {
  const { user } = useAuth()
  const [settling, setSettling] = useState<SuggestedTransfer | null>(null)

  if (loading) return <Spinner />
  if (error) return <ErrorMessage message={error} />
  if (!balances) return null

  const settled = balances.balances.every((b) => b.net_cents === 0)

  return (
    <div className="space-y-6">
      <section>
        <h2 className="mb-3 flex items-center gap-2 text-lg font-semibold">
          <Scale className="size-5 text-slate-400" />
          Saldos
        </h2>
        <ul className="divide-y divide-slate-200 overflow-hidden rounded-2xl border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900">
          {balances.balances.map((entry) => (
            <li key={entry.user.id} className="flex items-center gap-3 px-4 py-3">
              <Avatar user={entry.user} size="sm" />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">
                  {entry.user.id === user?.id ? 'Yo' : entry.user.display_name}
                </p>
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  Puso {money(entry.paid_cents, balances.currency)} · le tocó{' '}
                  {money(entry.owed_cents, balances.currency)}
                </p>
              </div>
              <SignedAmount cents={entry.net_cents} currency={balances.currency} />
            </li>
          ))}
        </ul>
      </section>

      <section>
        <h2 className="mb-3 flex items-center gap-2 text-lg font-semibold">
          <HandCoins className="size-5 text-slate-400" />
          Cómo saldar
        </h2>

        {settled ? (
          <EmptyState
            icon={<Check className="size-10 text-emerald-500" />}
            title="Están a mano"
            description="Nadie le debe nada a nadie en este grupo."
          />
        ) : (
          <ul className="space-y-2">
            {balances.transfers.map((transfer, index) => (
              <li key={`${transfer.from.id}-${transfer.to.id}-${index}`}>
                <Card className="flex flex-wrap items-center gap-3 p-4">
                  <Avatar user={transfer.from} size="sm" />
                  <span className="text-sm">
                    <strong>
                      {transfer.from.id === user?.id ? 'Vos' : transfer.from.display_name}
                    </strong>{' '}
                    le paga a{' '}
                    <strong>{transfer.to.id === user?.id ? 'vos' : transfer.to.display_name}</strong>
                  </span>
                  <Avatar user={transfer.to} size="sm" />
                  <span className="tabular ml-auto font-semibold">
                    {money(transfer.amount_cents, balances.currency)}
                  </span>
                  <Button size="sm" variant="secondary" onClick={() => setSettling(transfer)}>
                    Registrar pago
                  </Button>
                </Card>
              </li>
            ))}
          </ul>
        )}
      </section>

      <SettlementsHistory groupId={groupId} onChanged={onSettled} />

      <SettleModal
        groupId={groupId}
        transfer={settling}
        currency={balances.currency}
        onClose={() => setSettling(null)}
        onDone={() => {
          setSettling(null)
          onSettled()
        }}
      />
    </div>
  )
}

function SettleModal({
  groupId,
  transfer,
  currency,
  onClose,
  onDone,
}: {
  groupId: string
  transfer: SuggestedTransfer | null
  currency: string
  onClose: () => void
  onDone: () => void
}) {
  const toast = useToast()
  const [amount, setAmount] = useState('')
  const [note, setNote] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // El importe arranca con lo que sugiere la simplificación.
  const suggested = transfer ? (transfer.amount_cents / 100).toFixed(2) : ''

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault()
    if (!transfer) return

    const value = Math.round(Number((amount || suggested).replace(',', '.')) * 100)
    if (!Number.isFinite(value) || value <= 0) {
      setError('Poné un importe válido')
      return
    }

    setSaving(true)
    setError(null)
    try {
      await api.createSettlement(groupId, {
        from_user: transfer.from.id,
        to_user: transfer.to.id,
        amount_cents: value,
        note: note || undefined,
      })
      toast.success('Pago registrado')
      emitRefresh()
      setAmount('')
      setNote('')
      onDone()
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Modal open={Boolean(transfer)} onClose={onClose} title="Registrar pago">
      {transfer && (
        <form onSubmit={handleSubmit} className="space-y-4">
          {error && <ErrorMessage message={error} />}

          <div className="flex items-center justify-center gap-3 rounded-xl bg-slate-50 py-4 dark:bg-slate-800/60">
            <div className="text-center">
              <Avatar user={transfer.from} size="md" className="mx-auto" />
              <p className="mt-1 text-xs">{transfer.from.display_name}</p>
            </div>
            <span className="text-2xl text-slate-400">→</span>
            <div className="text-center">
              <Avatar user={transfer.to} size="md" className="mx-auto" />
              <p className="mt-1 text-xs">{transfer.to.display_name}</p>
            </div>
          </div>

          <Field label={`Importe (${currency})`}>
            <Input
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder={suggested}
              inputMode="decimal"
              className="tabular text-lg font-semibold"
              autoFocus
            />
          </Field>

          <Field label="Nota (opcional)">
            <Input
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder="Transferencia, efectivo…"
            />
          </Field>

          <div className="flex justify-end gap-2">
            <Button type="button" variant="secondary" onClick={onClose}>
              Cancelar
            </Button>
            <Button type="submit" loading={saving}>
              Registrar
            </Button>
          </div>
        </form>
      )}
    </Modal>
  )
}

function SettlementsHistory({ groupId, onChanged }: { groupId: string; onChanged: () => void }) {
  const toast = useToast()
  const { data, loading, reload } = useAsync(() => api.settlements(groupId), [groupId])

  if (loading || !data || data.length === 0) return null

  async function remove(id: string) {
    try {
      await api.deleteSettlement(id)
      toast.success('Pago eliminado')
      reload()
      onChanged()
    } catch (err) {
      toast.error(errorMessage(err))
    }
  }

  return (
    <section>
      <h2 className="mb-3 text-lg font-semibold">Pagos registrados</h2>
      <ul className="divide-y divide-slate-200 overflow-hidden rounded-2xl border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900">
        {data.map((settlement) => (
          <li key={settlement.id} className="flex items-center gap-3 px-4 py-3 text-sm">
            <Avatar user={settlement.from} size="xs" />
            <span className="min-w-0 flex-1 truncate">
              {settlement.from.display_name} → {settlement.to.display_name}
              <span className="ml-2 text-xs text-slate-500">
                {formatDate(settlement.settled_at)}
                {settlement.note ? ` · ${settlement.note}` : ''}
              </span>
            </span>
            <span className="tabular font-medium">
              {money(settlement.amount_cents, settlement.currency)}
            </span>
            <button
              type="button"
              onClick={() => remove(settlement.id)}
              aria-label="Eliminar pago"
              className="rounded-lg p-1 text-slate-400 transition hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950/50"
            >
              <Trash2 className="size-4" />
            </button>
          </li>
        ))}
      </ul>
    </section>
  )
}

// ---------------------------------------------------------------------------
// Miembros
// ---------------------------------------------------------------------------

function MembersTab({ detail, onChanged }: { detail: GroupDetail; onChanged: () => void }) {
  const { user } = useAuth()
  const toast = useToast()
  const navigate = useNavigate()
  const [email, setEmail] = useState('')
  const [adding, setAdding] = useState(false)
  const [removing, setRemoving] = useState<Member | null>(null)
  const [busy, setBusy] = useState(false)

  async function addMember(event: React.FormEvent) {
    event.preventDefault()
    setAdding(true)
    try {
      await api.addMember(detail.id, email.trim())
      toast.success('Miembro agregado')
      setEmail('')
      onChanged()
    } catch (err) {
      toast.error(errorMessage(err))
    } finally {
      setAdding(false)
    }
  }

  async function confirmRemove() {
    if (!removing) return
    setBusy(true)
    try {
      await api.removeMember(detail.id, removing.user.id)
      const wasMe = removing.user.id === user?.id
      toast.success(wasMe ? 'Saliste del grupo' : 'Miembro eliminado')
      setRemoving(null)
      if (wasMe) navigate('/grupos')
      else onChanged()
    } catch (err) {
      toast.error(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="space-y-5">
      <Card className="p-4">
        <form onSubmit={addMember} className="flex flex-wrap gap-2">
          <Input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="email@dequien.querés.sumar"
            className="min-w-0 flex-1"
            required
          />
          <Button type="submit" loading={adding} icon={<UserPlus className="size-4" />}>
            Agregar
          </Button>
        </form>
        <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
          Tiene que tener cuenta en TarascaPay. Si no, pasale el código{' '}
          <strong className="tabular">{detail.invite_code}</strong>.
        </p>
      </Card>

      <ul className="divide-y divide-slate-200 overflow-hidden rounded-2xl border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900">
        {detail.members.map((member) => (
          <li key={member.user.id} className="flex items-center gap-3 px-4 py-3">
            <Avatar user={member.user} size="sm" />
            <div className="min-w-0 flex-1">
              <p className="flex items-center gap-2 truncate text-sm font-medium">
                {member.user.id === user?.id ? 'Yo' : member.user.display_name}
                {member.role === 'owner' && <Badge tone="brand">Admin</Badge>}
              </p>
              <p className="truncate text-xs text-slate-500 dark:text-slate-400">
                {member.user.email}
              </p>
            </div>
            <SignedAmount cents={member.balance_cents} currency={detail.currency} />
            {(detail.role === 'owner' || member.user.id === user?.id) && (
              <button
                type="button"
                onClick={() => setRemoving(member)}
                aria-label={
                  member.user.id === user?.id ? 'Salir del grupo' : `Sacar a ${member.user.display_name}`
                }
                className="rounded-lg p-1.5 text-slate-400 transition hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950/50"
              >
                <Trash2 className="size-4" />
              </button>
            )}
          </li>
        ))}
      </ul>

      <ConfirmDialog
        open={Boolean(removing)}
        title={removing?.user.id === user?.id ? 'Salir del grupo' : 'Sacar del grupo'}
        message={
          removing?.user.id === user?.id
            ? '¿Seguro que querés salir? Necesitás tener saldo cero en el grupo.'
            : `¿Sacar a ${removing?.user.display_name}? Sólo se puede si su saldo está en cero.`
        }
        confirmLabel={removing?.user.id === user?.id ? 'Salir' : 'Sacar'}
        danger
        loading={busy}
        onConfirm={confirmRemove}
        onCancel={() => setRemoving(null)}
      />
    </div>
  )
}

// ---------------------------------------------------------------------------
// Actividad
// ---------------------------------------------------------------------------

const ACTIVITY_TEXT: Record<string, (payload: Record<string, unknown>) => string> = {
  group_created: () => 'creó el grupo',
  member_joined: () => 'se unió al grupo',
  member_added: (p) => `agregó a ${String(p.member ?? 'alguien')}`,
  expense_created: (p) => `agregó "${String(p.description ?? 'un gasto')}"`,
  expense_updated: (p) => `editó "${String(p.description ?? 'un gasto')}"`,
  expense_deleted: (p) => `eliminó "${String(p.description ?? 'un gasto')}"`,
  settlement_created: () => 'registró un pago',
}

function ActivityTab({ groupId }: { groupId: string }) {
  const { data, loading, error } = useAsync(() => api.activity(groupId), [groupId])

  if (loading && !data) return <Spinner />
  if (error) return <ErrorMessage message={error} />
  if (!data || data.length === 0) {
    return <EmptyState title="Sin actividad" description="Acá va a aparecer todo lo que pase en el grupo." />
  }

  return (
    <ul className="space-y-2">
      {data.map((item) => (
        <li key={item.id} className="flex items-center gap-3 rounded-xl bg-white px-4 py-3 text-sm ring-1 ring-slate-200 dark:bg-slate-900 dark:ring-slate-800">
          <Avatar user={item.actor} size="xs" />
          <span className="min-w-0 flex-1">
            <strong>{item.actor.display_name}</strong>{' '}
            {ACTIVITY_TEXT[item.kind]?.(item.payload) ?? item.kind}
          </span>
          <span className="shrink-0 text-xs text-slate-500 dark:text-slate-400">
            {relativeTime(item.created_at)}
          </span>
        </li>
      ))}
    </ul>
  )
}
