// Alta y edición de gastos, con reparto configurable y comprobantes.

import { useEffect, useMemo, useState } from 'react'
import { Paperclip, Trash2 } from 'lucide-react'

import { api } from '../lib/api'
import { useAuth } from '../lib/auth'
import { centsToInput, money, parseAmount, today } from '../lib/format'
import { emitRefresh, errorMessage } from '../lib/hooks'
import { useToast } from '../lib/toast'
import type { Attachment, Category, Expense, Group, Member, SplitType, UserBrief } from '../lib/types'
import { Avatar, Button, ErrorMessage, Field, Input, Modal, Select, Spinner, Textarea, cn } from './ui'

const SPLIT_LABELS: Record<SplitType, string> = {
  equal: 'Partes iguales',
  exact: 'Importes exactos',
  percentage: 'Porcentajes',
  shares: 'Por partes',
}

interface Props {
  open: boolean
  onClose: () => void
  onSaved: (expense: Expense) => void
  /** Preselecciona un grupo (desde la pantalla del grupo). */
  groupId?: string | null
  /** Si viene, el formulario edita ese gasto en lugar de crear uno nuevo. */
  expense?: Expense | null
}

export function ExpenseFormModal({ open, onClose, onSaved, groupId, expense }: Props) {
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={expense ? 'Editar gasto' : 'Nuevo gasto'}
      wide
    >
      {open && (
        <ExpenseForm
          key={expense?.id ?? 'nuevo'}
          onClose={onClose}
          onSaved={onSaved}
          groupId={groupId}
          expense={expense}
        />
      )}
    </Modal>
  )
}

function ExpenseForm({ onClose, onSaved, groupId, expense }: Omit<Props, 'open'>) {
  const { user } = useAuth()
  const toast = useToast()

  const [groups, setGroups] = useState<Group[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const [members, setMembers] = useState<Member[]>([])
  const [ready, setReady] = useState(false)

  // Campos del formulario.
  const [selectedGroup, setSelectedGroup] = useState<string>(
    expense?.group_id ?? groupId ?? '',
  )
  const [description, setDescription] = useState(expense?.description ?? '')
  const [amount, setAmount] = useState(expense ? centsToInput(expense.amount_cents) : '')
  const [date, setDate] = useState(expense?.expense_date ?? today())
  const [categoryId, setCategoryId] = useState<number | null>(expense?.category.id ?? null)
  const [notes, setNotes] = useState(expense?.notes ?? '')
  const [paidBy, setPaidBy] = useState(expense?.paid_by.id ?? user?.id ?? '')
  const [splitType, setSplitType] = useState<SplitType>(
    expense && expense.group_id ? expense.split_type : 'equal',
  )
  /** Participantes seleccionados y el valor crudo de cada uno. */
  const [participants, setParticipants] = useState<Record<string, boolean>>({})
  const [values, setValues] = useState<Record<string, string>>({})
  const [attachments, setAttachments] = useState<Attachment[]>(expense?.attachments ?? [])

  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const amountCents = parseAmount(amount) ?? 0
  const isPersonal = selectedGroup === ''

  // Carga inicial de grupos y categorías.
  useEffect(() => {
    let cancelled = false
    Promise.all([api.groups(), api.categories()])
      .then(([g, c]) => {
        if (cancelled) return
        setGroups(g.filter((group) => !group.archived))
        setCategories(c)
        setCategoryId((current) => current ?? c.find((cat) => cat.slug === 'general')?.id ?? null)
      })
      .catch((err) => !cancelled && setError(errorMessage(err)))
      .finally(() => !cancelled && setReady(true))
    return () => {
      cancelled = true
    }
  }, [])

  // Miembros del grupo elegido: definen quién puede participar del reparto.
  useEffect(() => {
    if (!selectedGroup) {
      setMembers([])
      return
    }

    let cancelled = false
    api
      .group(selectedGroup)
      .then((detail) => {
        if (cancelled) return
        setMembers(detail.members)

        // Al editar se respetan los participantes guardados; al crear, entran todos.
        const existing = expense?.splits ?? []
        const next: Record<string, boolean> = {}
        const nextValues: Record<string, string> = {}

        for (const member of detail.members) {
          const saved = existing.find((s) => s.user.id === member.user.id)
          next[member.user.id] = existing.length > 0 ? Boolean(saved) : true
          if (saved) nextValues[member.user.id] = rawValue(saved.split_value, splitType)
        }

        setParticipants(next)
        setValues(nextValues)
      })
      .catch((err) => !cancelled && setError(errorMessage(err)))

    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedGroup])

  const chosen = useMemo(
    () => members.filter((m) => participants[m.user.id]),
    [members, participants],
  )

  // Cuánto falta (o sobra) para que el reparto cierre.
  const splitStatus = useMemo(
    () => computeSplitStatus(splitType, chosen, values, amountCents),
    [splitType, chosen, values, amountCents],
  )

  async function handleUpload(files: FileList | null) {
    if (!files?.length) return
    try {
      const uploaded = await Promise.all(Array.from(files).map((file) => api.upload(file)))
      setAttachments((current) => [...current, ...uploaded])
    } catch (err) {
      toast.error(errorMessage(err))
    }
  }

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault()
    setError(null)

    if (!description.trim()) return setError('Poné una descripción')
    if (amountCents <= 0) return setError('El importe tiene que ser mayor a cero')
    if (!isPersonal && chosen.length === 0) return setError('Elegí al menos un participante')
    if (!isPersonal && splitStatus.error) return setError(splitStatus.error)

    const body = {
      group_id: isPersonal ? null : selectedGroup,
      description: description.trim(),
      notes: notes.trim() || null,
      amount_cents: amountCents,
      category_id: categoryId ?? undefined,
      expense_date: date,
      paid_by: isPersonal ? undefined : paidBy,
      split_type: isPersonal ? ('equal' as SplitType) : splitType,
      splits: isPersonal
        ? []
        : chosen.map((m) => ({
            user_id: m.user.id,
            value: splitValueFor(splitType, values[m.user.id], amountCents, chosen.length),
          })),
      attachment_ids: attachments.map((a) => a.id),
    }

    setSaving(true)
    try {
      const saved = expense
        ? await api.updateExpense(expense.id, body)
        : await api.createExpense(body)
      toast.success(expense ? 'Gasto actualizado' : 'Gasto agregado')
      emitRefresh()
      onSaved(saved)
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  if (!ready) return <Spinner />

  return (
    <form onSubmit={handleSubmit} className="space-y-5">
      {error && <ErrorMessage message={error} />}

      <div className="grid gap-4 sm:grid-cols-2">
        <Field label="Descripción" className="sm:col-span-2">
          <Input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Cena en el centro"
            autoFocus
            maxLength={200}
          />
        </Field>

        <Field label="Importe">
          <Input
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            inputMode="decimal"
            placeholder="0,00"
            className="tabular text-lg font-semibold"
          />
        </Field>

        <Field label="Fecha">
          <Input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
        </Field>

        <Field label="Grupo" hint={isPersonal ? 'Sólo vos vas a ver este gasto' : undefined}>
          <Select
            value={selectedGroup}
            onChange={(e) => setSelectedGroup(e.target.value)}
            disabled={Boolean(expense)}
          >
            <option value="">Gasto personal</option>
            {groups.map((group) => (
              <option key={group.id} value={group.id}>
                {group.emoji} {group.name}
              </option>
            ))}
          </Select>
        </Field>

        <Field label="Categoría">
          <Select
            value={categoryId ?? ''}
            onChange={(e) => setCategoryId(Number(e.target.value))}
          >
            {categories.map((category) => (
              <option key={category.id} value={category.id}>
                {category.icon} {category.name}
              </option>
            ))}
          </Select>
        </Field>
      </div>

      {!isPersonal && members.length > 0 && (
        <>
          <Field label="Pagó">
            <Select value={paidBy} onChange={(e) => setPaidBy(e.target.value)}>
              {members.map((member) => (
                <option key={member.user.id} value={member.user.id}>
                  {member.user.id === user?.id ? 'Yo' : member.user.display_name}
                </option>
              ))}
            </Select>
          </Field>

          <div>
            <span className="label">Cómo se divide</span>
            <div className="mb-3 grid grid-cols-2 gap-2 sm:grid-cols-4">
              {(Object.keys(SPLIT_LABELS) as SplitType[]).map((type) => (
                <button
                  key={type}
                  type="button"
                  onClick={() => {
                    setSplitType(type)
                    setValues({})
                  }}
                  className={cn(
                    'rounded-xl px-3 py-2 text-xs font-medium transition',
                    splitType === type
                      ? 'bg-brand-600 text-white'
                      : 'bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700',
                  )}
                >
                  {SPLIT_LABELS[type]}
                </button>
              ))}
            </div>

            <div className="divide-y divide-slate-200 overflow-hidden rounded-xl border border-slate-200 dark:divide-slate-800 dark:border-slate-800">
              {members.map((member) => (
                <ParticipantRow
                  key={member.user.id}
                  user={member.user}
                  isMe={member.user.id === user?.id}
                  checked={Boolean(participants[member.user.id])}
                  onToggle={(checked) =>
                    setParticipants((current) => ({ ...current, [member.user.id]: checked }))
                  }
                  splitType={splitType}
                  value={values[member.user.id] ?? ''}
                  onValueChange={(value) =>
                    setValues((current) => ({ ...current, [member.user.id]: value }))
                  }
                  computed={splitStatus.shares[member.user.id] ?? 0}
                  currency={groups.find((g) => g.id === selectedGroup)?.currency ?? 'ARS'}
                />
              ))}
            </div>

            <p
              className={cn(
                'mt-2 text-xs',
                splitStatus.error
                  ? 'text-rose-600 dark:text-rose-400'
                  : 'text-slate-500 dark:text-slate-400',
              )}
            >
              {splitStatus.error ?? splitStatus.hint}
            </p>
          </div>
        </>
      )}

      <Field label="Notas (opcional)">
        <Textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={2}
          placeholder="Detalle, quién falta pagar, etc."
        />
      </Field>

      <div>
        <span className="label">Comprobantes</span>
        <div className="flex flex-wrap items-center gap-2">
          {attachments.map((attachment) => (
            <div
              key={attachment.id}
              className="group relative size-20 overflow-hidden rounded-xl border border-slate-200 dark:border-slate-700"
            >
              {attachment.mime_type.startsWith('image/') ? (
                <img
                  src={api.fileUrl(attachment.id)}
                  alt={attachment.file_name}
                  className="size-full object-cover"
                />
              ) : (
                <div className="flex size-full items-center justify-center bg-slate-100 text-xs dark:bg-slate-800">
                  PDF
                </div>
              )}
              <button
                type="button"
                aria-label={`Quitar ${attachment.file_name}`}
                onClick={() =>
                  setAttachments((current) => current.filter((a) => a.id !== attachment.id))
                }
                className="absolute top-1 right-1 rounded-md bg-slate-900/70 p-1 text-white opacity-0 transition group-hover:opacity-100 focus:opacity-100"
              >
                <Trash2 className="size-3.5" />
              </button>
            </div>
          ))}

          <label className="flex size-20 cursor-pointer flex-col items-center justify-center gap-1 rounded-xl border border-dashed border-slate-300 text-xs text-slate-500 transition hover:border-brand-500 hover:text-brand-600 dark:border-slate-700">
            <Paperclip className="size-4" />
            Subir
            <input
              type="file"
              accept="image/*,application/pdf"
              multiple
              className="sr-only"
              onChange={(e) => {
                void handleUpload(e.target.files)
                e.target.value = ''
              }}
            />
          </label>
        </div>
      </div>

      <div className="flex justify-end gap-2 pt-2">
        <Button type="button" variant="secondary" onClick={onClose}>
          Cancelar
        </Button>
        <Button type="submit" loading={saving}>
          {expense ? 'Guardar cambios' : 'Agregar gasto'}
        </Button>
      </div>
    </form>
  )
}

function ParticipantRow({
  user,
  isMe,
  checked,
  onToggle,
  splitType,
  value,
  onValueChange,
  computed,
  currency,
}: {
  user: UserBrief
  isMe: boolean
  checked: boolean
  onToggle: (checked: boolean) => void
  splitType: SplitType
  value: string
  onValueChange: (value: string) => void
  computed: number
  currency: string
}) {
  return (
    <div className="flex items-center gap-3 bg-white px-3 py-2.5 dark:bg-slate-900">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onToggle(e.target.checked)}
        aria-label={`Incluir a ${user.display_name}`}
        className="size-4 rounded border-slate-300 text-brand-600 focus:ring-brand-500"
      />
      <Avatar user={user} size="sm" />
      <span className="min-w-0 flex-1 truncate text-sm">
        {isMe ? 'Yo' : user.display_name}
      </span>

      {checked && splitType !== 'equal' && (
        <div className="flex items-center gap-1">
          <input
            value={value}
            onChange={(e) => onValueChange(e.target.value)}
            inputMode="decimal"
            placeholder="0"
            aria-label={`Valor de ${user.display_name}`}
            className="tabular w-20 rounded-lg border border-slate-300 px-2 py-1 text-right text-sm dark:border-slate-700 dark:bg-slate-950"
          />
          <span className="w-4 text-xs text-slate-400">
            {splitType === 'percentage' ? '%' : splitType === 'shares' ? '×' : ''}
          </span>
        </div>
      )}

      <span className="tabular w-24 text-right text-sm font-medium text-slate-600 dark:text-slate-300">
        {checked ? money(computed, currency) : '—'}
      </span>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Cálculo del reparto (previsualización en vivo, el servidor lo recalcula)
// ---------------------------------------------------------------------------

interface SplitStatus {
  shares: Record<string, number>
  error: string | null
  hint: string
}

function computeSplitStatus(
  splitType: SplitType,
  chosen: Member[],
  values: Record<string, string>,
  amountCents: number,
): SplitStatus {
  const shares: Record<string, number> = {}
  if (chosen.length === 0) return { shares, error: null, hint: 'Elegí quiénes participan' }

  if (splitType === 'equal') {
    // Mismo criterio que el servidor: el resto se reparte de a un centavo.
    const base = Math.floor(amountCents / chosen.length)
    let leftover = amountCents - base * chosen.length
    for (const member of chosen) {
      shares[member.user.id] = base + (leftover > 0 ? 1 : 0)
      if (leftover > 0) leftover--
    }
    return {
      shares,
      error: null,
      hint: `${money(Math.floor(amountCents / chosen.length))} por persona (${chosen.length})`,
    }
  }

  const numbers = chosen.map((m) => parseNumber(values[m.user.id]))

  if (splitType === 'exact') {
    let total = 0
    chosen.forEach((member, index) => {
      const cents = Math.round(numbers[index] * 100)
      shares[member.user.id] = cents
      total += cents
    })
    const diff = amountCents - total
    return {
      shares,
      error: diff !== 0 ? `Faltan asignar ${money(diff)} (los importes deben sumar el total)` : null,
      hint: 'Los importes tienen que sumar exactamente el total',
    }
  }

  if (splitType === 'percentage') {
    const totalPct = numbers.reduce((sum, n) => sum + n, 0)
    chosen.forEach((member, index) => {
      shares[member.user.id] = Math.round((amountCents * numbers[index]) / 100)
    })
    const diff = Math.round((100 - totalPct) * 100) / 100
    return {
      shares,
      error: diff !== 0 ? `Los porcentajes suman ${totalPct}% (falta ${diff}%)` : null,
      hint: 'Los porcentajes tienen que sumar 100 %',
    }
  }

  // shares
  const totalShares = numbers.reduce((sum, n) => sum + n, 0)
  if (totalShares <= 0) {
    return { shares, error: 'Asigná al menos una parte', hint: '' }
  }
  chosen.forEach((member, index) => {
    shares[member.user.id] = Math.round((amountCents * numbers[index]) / totalShares)
  })
  return { shares, error: null, hint: `${totalShares} partes en total` }
}

function parseNumber(raw: string | undefined): number {
  if (!raw) return 0
  const value = Number(raw.replace(',', '.').replace(/[^\d.-]/g, ''))
  return Number.isFinite(value) ? value : 0
}

/** Convierte lo que se ve en pantalla al entero que espera la API. */
function splitValueFor(
  splitType: SplitType,
  raw: string | undefined,
  amountCents: number,
  participantCount: number,
): number {
  const value = parseNumber(raw)
  switch (splitType) {
    case 'exact':
      return Math.round(value * 100)
    case 'percentage':
      // La API usa puntos básicos: 33,33 % → 3333.
      return Math.round(value * 100)
    case 'shares':
      return Math.round(value)
    default:
      // En partes iguales el valor no se usa.
      void amountCents
      void participantCount
      return 0
  }
}

/** Inverso de `splitValueFor`, para precargar el formulario al editar. */
function rawValue(splitValue: number, splitType: SplitType): string {
  switch (splitType) {
    case 'exact':
    case 'percentage':
      return (splitValue / 100).toString()
    case 'shares':
      return splitValue.toString()
    default:
      return ''
  }
}
