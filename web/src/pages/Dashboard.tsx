// Panel principal: cuánto debés, cuánto te deben y últimos movimientos.

import { Link } from 'react-router-dom'
import { ArrowRight, TrendingDown, TrendingUp, Users, Wallet } from 'lucide-react'

import { api } from '../lib/api'
import { useAuth } from '../lib/auth'
import { money } from '../lib/format'
import { useAsync } from '../lib/hooks'
import { ExpenseList } from '../components/ExpenseList'
import { Avatar, Card, ErrorMessage, PageTitle, SignedAmount, Spinner } from '../components/ui'

export function DashboardPage() {
  const { user } = useAuth()
  const { data, loading, error } = useAsync(() => api.dashboard(), [])

  if (loading && !data) return <Spinner />
  if (error) return <ErrorMessage message={error} />
  if (!data) return null

  const firstName = user?.display_name.split(' ')[0] ?? ''

  return (
    <>
      <PageTitle
        title={`Hola, ${firstName} 👋`}
        subtitle="Este es el estado de tus cuentas."
      />

      {/* Resumen de saldos */}
      <div className="mb-6 grid gap-3 sm:grid-cols-3">
        <Card className="p-5">
          <div className="mb-2 flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
            <TrendingUp className="size-4 text-emerald-500" />
            Te deben
          </div>
          <p className="tabular text-2xl font-bold text-emerald-600 dark:text-emerald-400">
            {money(data.you_are_owed_cents, data.currency)}
          </p>
        </Card>

        <Card className="p-5">
          <div className="mb-2 flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
            <TrendingDown className="size-4 text-rose-500" />
            Debés
          </div>
          <p className="tabular text-2xl font-bold text-rose-600 dark:text-rose-400">
            {money(data.you_owe_cents, data.currency)}
          </p>
        </Card>

        <Card className="p-5">
          <div className="mb-2 flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
            <Wallet className="size-4 text-slate-400" />
            Balance neto
          </div>
          <SignedAmount cents={data.net_cents} currency={data.currency} className="text-2xl" />
        </Card>
      </div>

      {/* Gasto del mes */}
      <div className="mb-6 grid gap-3 sm:grid-cols-3">
        <Card className="p-4">
          <p className="text-xs text-slate-500 dark:text-slate-400">Personal este mes</p>
          <p className="tabular mt-1 text-lg font-semibold">
            {money(data.personal_this_month_cents, data.currency)}
          </p>
        </Card>
        <Card className="p-4">
          <p className="text-xs text-slate-500 dark:text-slate-400">En grupos este mes</p>
          <p className="tabular mt-1 text-lg font-semibold">
            {money(data.group_this_month_cents, data.currency)}
          </p>
        </Card>
        <Card className="p-4">
          <p className="text-xs text-slate-500 dark:text-slate-400">Grupos activos</p>
          <p className="tabular mt-1 flex items-center gap-2 text-lg font-semibold">
            <Users className="size-4 text-slate-400" />
            {data.active_groups}
          </p>
        </Card>
      </div>

      {/* Deudas persona por persona */}
      {data.debts.length > 0 && (
        <section className="mb-8">
          <h2 className="mb-3 text-lg font-semibold">Con quién estás en deuda</h2>
          <ul className="divide-y divide-slate-200 overflow-hidden rounded-2xl border border-slate-200 bg-white dark:divide-slate-800 dark:border-slate-800 dark:bg-slate-900">
            {data.debts.map((debt) => (
              <li key={debt.user.id} className="flex items-center gap-3 px-4 py-3">
                <Avatar user={debt.user} size="sm" />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium">{debt.user.display_name}</p>
                  <p className="text-xs text-slate-500 dark:text-slate-400">
                    {debt.net_cents > 0 ? 'te debe' : 'le debés'}
                  </p>
                </div>
                <SignedAmount cents={debt.net_cents} currency={data.currency} />
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Últimos gastos */}
      <section>
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-lg font-semibold">Movimientos recientes</h2>
          <Link
            to="/personal"
            className="flex items-center gap-1 text-sm font-medium text-brand-600 hover:underline"
          >
            Ver todo <ArrowRight className="size-4" />
          </Link>
        </div>
        <ExpenseList
          expenses={data.recent_expenses}
          showGroup
          emptyTitle="Todavía no cargaste gastos"
          emptyDescription="Tocá el botón + para agregar el primero."
        />
      </section>
    </>
  )
}
