// Listado de grupos, alta de grupo y unirse con código.

import { useState } from 'react'
import { Link } from 'react-router-dom'
import { LogIn, Plus, Users } from 'lucide-react'

import { api } from '../lib/api'
import { money } from '../lib/format'
import { errorMessage, useAsync } from '../lib/hooks'
import { useToast } from '../lib/toast'
import type { GroupKind } from '../lib/types'
import {
  Badge,
  Button,
  Card,
  EmptyState,
  ErrorMessage,
  Field,
  Input,
  Modal,
  PageTitle,
  Select,
  SignedAmount,
  Spinner,
  Textarea,
} from '../components/ui'

const KINDS: { value: GroupKind; label: string; emoji: string }[] = [
  { value: 'trip', label: 'Viaje', emoji: '✈️' },
  { value: 'home', label: 'Casa compartida', emoji: '🏠' },
  { value: 'couple', label: 'Pareja', emoji: '❤️' },
  { value: 'event', label: 'Evento', emoji: '🎉' },
  { value: 'other', label: 'Otro', emoji: '🌎' },
]

export function GroupsPage() {
  const { data: groups, loading, error, reload } = useAsync(() => api.groups(), [])
  const [creating, setCreating] = useState(false)
  const [joining, setJoining] = useState(false)

  const active = groups?.filter((g) => !g.archived) ?? []
  const archived = groups?.filter((g) => g.archived) ?? []

  return (
    <>
      <PageTitle
        title="Grupos"
        subtitle="Viajes, casas compartidas y todo lo que dividas con otros."
        action={
          <div className="flex gap-2">
            <Button variant="secondary" icon={<LogIn className="size-4" />} onClick={() => setJoining(true)}>
              Unirme
            </Button>
            <Button icon={<Plus className="size-4" />} onClick={() => setCreating(true)}>
              Nuevo grupo
            </Button>
          </div>
        }
      />

      {error && <ErrorMessage message={error} />}
      {loading && !groups && <Spinner />}

      {groups && active.length === 0 && archived.length === 0 && (
        <EmptyState
          icon={<Users className="size-10" />}
          title="Todavía no tenés grupos"
          description="Creá uno para tu próximo viaje o unite con el código que te pasaron."
          action={
            <div className="flex gap-2">
              <Button onClick={() => setCreating(true)}>Crear grupo</Button>
              <Button variant="secondary" onClick={() => setJoining(true)}>
                Tengo un código
              </Button>
            </div>
          }
        />
      )}

      <div className="grid gap-3 sm:grid-cols-2">
        {active.map((group) => (
          <Link key={group.id} to={`/grupos/${group.id}`}>
            <Card className="h-full p-5 transition hover:border-brand-400 hover:shadow-md">
              <div className="flex items-start gap-3">
                <span aria-hidden className="text-3xl">
                  {group.emoji}
                </span>
                <div className="min-w-0 flex-1">
                  <p className="truncate font-semibold">{group.name}</p>
                  <p className="text-xs text-slate-500 dark:text-slate-400">
                    {group.member_count} {group.member_count === 1 ? 'persona' : 'personas'} ·{' '}
                    {group.expense_count} {group.expense_count === 1 ? 'gasto' : 'gastos'}
                  </p>
                </div>
                {group.role === 'owner' && <Badge tone="brand">Admin</Badge>}
              </div>

              <div className="mt-4 flex items-end justify-between">
                <div>
                  <p className="text-xs text-slate-500 dark:text-slate-400">Total gastado</p>
                  <p className="tabular font-semibold">
                    {money(group.total_spent_cents, group.currency)}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-xs text-slate-500 dark:text-slate-400">
                    {group.my_balance_cents > 0
                      ? 'Te deben'
                      : group.my_balance_cents < 0
                        ? 'Debés'
                        : 'Al día'}
                  </p>
                  <SignedAmount cents={group.my_balance_cents} currency={group.currency} />
                </div>
              </div>
            </Card>
          </Link>
        ))}
      </div>

      {archived.length > 0 && (
        <section className="mt-8">
          <h2 className="mb-3 text-sm font-semibold text-slate-500 dark:text-slate-400">
            Archivados
          </h2>
          <div className="grid gap-3 sm:grid-cols-2">
            {archived.map((group) => (
              <Link key={group.id} to={`/grupos/${group.id}`}>
                <Card className="flex items-center gap-3 p-4 opacity-70 transition hover:opacity-100">
                  <span aria-hidden className="text-2xl">
                    {group.emoji}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-sm font-medium">{group.name}</span>
                  <Badge>Archivado</Badge>
                </Card>
              </Link>
            ))}
          </div>
        </section>
      )}

      <CreateGroupModal open={creating} onClose={() => setCreating(false)} onDone={reload} />
      <JoinGroupModal open={joining} onClose={() => setJoining(false)} onDone={reload} />
    </>
  )
}

function CreateGroupModal({
  open,
  onClose,
  onDone,
}: {
  open: boolean
  onClose: () => void
  onDone: () => void
}) {
  const toast = useToast()
  const [name, setName] = useState('')
  const [kind, setKind] = useState<GroupKind>('trip')
  const [description, setDescription] = useState('')
  const [currency, setCurrency] = useState('ARS')
  const [startDate, setStartDate] = useState('')
  const [endDate, setEndDate] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault()
    setError(null)
    setSaving(true)
    try {
      await api.createGroup({
        name,
        kind,
        currency,
        description: description || undefined,
        start_date: startDate || null,
        end_date: endDate || null,
      })
      toast.success('Grupo creado')
      onDone()
      onClose()
      setName('')
      setDescription('')
      setStartDate('')
      setEndDate('')
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="Nuevo grupo">
      <form onSubmit={handleSubmit} className="space-y-4">
        {error && <ErrorMessage message={error} />}

        <Field label="Nombre">
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Viaje a Bariloche"
            required
            autoFocus
            maxLength={80}
          />
        </Field>

        <div>
          <span className="label">Tipo</span>
          <div className="grid grid-cols-3 gap-2 sm:grid-cols-5">
            {KINDS.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => setKind(option.value)}
                className={`flex flex-col items-center gap-1 rounded-xl px-2 py-3 text-xs font-medium transition ${
                  kind === option.value
                    ? 'bg-brand-600 text-white'
                    : 'bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-300'
                }`}
              >
                <span className="text-xl">{option.emoji}</span>
                {option.label}
              </button>
            ))}
          </div>
        </div>

        <Field label="Moneda" hint="Todos los gastos del grupo usan esta moneda.">
          <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
            {['ARS', 'USD', 'EUR', 'BRL', 'CLP', 'UYU', 'MXN', 'COP', 'PEN', 'GBP'].map((code) => (
              <option key={code} value={code}>
                {code}
              </option>
            ))}
          </Select>
        </Field>

        {kind === 'trip' && (
          <div className="grid grid-cols-2 gap-3">
            <Field label="Desde">
              <Input
                type="date"
                value={startDate}
                max={endDate || undefined}
                onChange={(e) => setStartDate(e.target.value)}
              />
            </Field>
            <Field label="Hasta">
              <Input
                type="date"
                value={endDate}
                min={startDate || undefined}
                onChange={(e) => setEndDate(e.target.value)}
              />
            </Field>
          </div>
        )}

        <Field label="Descripción (opcional)">
          <Textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            rows={2}
            placeholder="Finde largo con los chicos"
          />
        </Field>

        <div className="flex justify-end gap-2 pt-1">
          <Button type="button" variant="secondary" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" loading={saving}>
            Crear grupo
          </Button>
        </div>
      </form>
    </Modal>
  )
}

function JoinGroupModal({
  open,
  onClose,
  onDone,
}: {
  open: boolean
  onClose: () => void
  onDone: () => void
}) {
  const toast = useToast()
  const [code, setCode] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault()
    setError(null)
    setSaving(true)
    try {
      const group = await api.joinGroup(code.trim())
      toast.success(`Te uniste a ${group.name}`)
      onDone()
      onClose()
      setCode('')
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Modal open={open} onClose={onClose} title="Unirme a un grupo">
      <form onSubmit={handleSubmit} className="space-y-4">
        {error && <ErrorMessage message={error} />}
        <Field label="Código de invitación" hint="Te lo pasa alguien que ya está en el grupo.">
          <Input
            value={code}
            onChange={(e) => setCode(e.target.value.toUpperCase())}
            placeholder="ABCD2345"
            className="tabular text-center text-lg tracking-widest uppercase"
            maxLength={12}
            required
            autoFocus
          />
        </Field>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="secondary" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" loading={saving}>
            Unirme
          </Button>
        </div>
      </form>
    </Modal>
  )
}
