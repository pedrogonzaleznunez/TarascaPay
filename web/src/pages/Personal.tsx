// Gastos personales y buscador sobre todos los gastos visibles.

import { useState } from 'react'
import { Plus, Search } from 'lucide-react'

import { api } from '../lib/api'
import { useAuth } from '../lib/auth'
import { money } from '../lib/format'
import { useAsync } from '../lib/hooks'
import { ExpenseFormModal } from '../components/ExpenseForm'
import { ExpenseList } from '../components/ExpenseList'
import {
  Button,
  Card,
  ErrorMessage,
  Input,
  PageTitle,
  Select,
  cn,
} from '../components/ui'

type Scope = 'personal' | 'todos'

export function PersonalPage() {
  const { user } = useAuth()
  const [scope, setScope] = useState<Scope>('personal')
  const [search, setSearch] = useState('')
  const [categoryId, setCategoryId] = useState<string>('')
  const [adding, setAdding] = useState(false)

  const categories = useAsync(() => api.categories(), [])

  const expenses = useAsync(
    () =>
      api.expenses({
        personal: scope === 'personal' ? true : undefined,
        search: search.trim() || undefined,
        category_id: categoryId ? Number(categoryId) : undefined,
        limit: 200,
      }),
    [scope, search, categoryId],
  )

  const total = (expenses.data ?? []).reduce(
    (sum, expense) => sum + (scope === 'personal' ? expense.amount_cents : myShare(expense, user?.id)),
    0,
  )

  const currency = expenses.data?.[0]?.currency ?? 'ARS'

  return (
    <>
      <PageTitle
        title="Mis gastos"
        subtitle="Lo que gastás por tu cuenta y lo que te toca en los grupos."
        action={
          <Button icon={<Plus className="size-4" />} onClick={() => setAdding(true)}>
            Nuevo gasto
          </Button>
        }
      />

      <div className="mb-4 flex gap-1 rounded-xl bg-slate-100 p-1 dark:bg-slate-800">
        {(['personal', 'todos'] as Scope[]).map((option) => (
          <button
            key={option}
            type="button"
            onClick={() => setScope(option)}
            className={cn(
              'flex-1 rounded-lg px-3 py-2 text-sm font-medium transition',
              scope === option
                ? 'bg-white text-slate-900 shadow-sm dark:bg-slate-900 dark:text-white'
                : 'text-slate-500 dark:text-slate-400',
            )}
          >
            {option === 'personal' ? 'Sólo personales' : 'Todos (incluye grupos)'}
          </button>
        ))}
      </div>

      <Card className="mb-4 flex flex-wrap gap-2 p-3">
        <div className="relative min-w-0 flex-1">
          <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-slate-400" />
          <Input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Buscar por descripción o nota…"
            className="pl-9"
          />
        </div>
        <Select
          value={categoryId}
          onChange={(e) => setCategoryId(e.target.value)}
          className="sm:w-52"
        >
          <option value="">Todas las categorías</option>
          {(categories.data ?? []).map((category) => (
            <option key={category.id} value={category.id}>
              {category.icon} {category.name}
            </option>
          ))}
        </Select>
      </Card>

      {expenses.data && expenses.data.length > 0 && (
        <p className="mb-3 text-sm text-slate-500 dark:text-slate-400">
          {expenses.data.length} {expenses.data.length === 1 ? 'gasto' : 'gastos'} ·{' '}
          <strong className="tabular text-slate-700 dark:text-slate-200">
            {money(total, currency)}
          </strong>{' '}
          {scope === 'personal' ? 'en total' : 'que te tocaron'}
        </p>
      )}

      {expenses.error && <ErrorMessage message={expenses.error} />}

      <ExpenseList
        expenses={expenses.data ?? []}
        loading={expenses.loading && !expenses.data}
        showGroup={scope === 'todos'}
        emptyTitle={search || categoryId ? 'Sin resultados' : 'Todavía no hay gastos'}
        emptyDescription={
          search || categoryId
            ? 'Probá con otra búsqueda o cambiá el filtro.'
            : 'Cargá tus gastos personales para llevar el control del mes.'
        }
        emptyAction={
          !search && !categoryId ? <Button onClick={() => setAdding(true)}>Agregar gasto</Button> : undefined
        }
      />

      <ExpenseFormModal
        open={adding}
        onClose={() => setAdding(false)}
        onSaved={() => {
          setAdding(false)
          expenses.reload()
        }}
      />
    </>
  )
}

/**
 * Cuánto de un gasto le corresponde pagar al usuario actual.
 *
 * La API devuelve `my_net = lo_que_puse − mi_parte`, así que la parte se
 * despeja sabiendo si el usuario fue quien pagó.
 */
function myShare(
  expense: { amount_cents: number; my_net_cents: number; paid_by: { id: string } },
  userId: string | undefined,
): number {
  const paid = expense.paid_by.id === userId ? expense.amount_cents : 0
  return paid - expense.my_net_cents
}
