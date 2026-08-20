// Resumen de gastos: por categoría y por mes.

import { useState } from 'react'

import { api } from '../lib/api'
import { formatMonth, money } from '../lib/format'
import { useAsync } from '../lib/hooks'
import { Card, EmptyState, ErrorMessage, PageTitle, Select, Spinner, cn } from '../components/ui'

export function StatsPage() {
  const [scope, setScope] = useState<string>('all')
  const [months, setMonths] = useState(6)

  const groups = useAsync(() => api.groups(), [])
  const stats = useAsync(
    () =>
      api.stats({
        group_id: scope !== 'all' && scope !== 'personal' ? scope : undefined,
        personal: scope === 'personal' ? true : undefined,
        months,
      }),
    [scope, months],
  )

  return (
    <>
      <PageTitle title="Resumen" subtitle="En qué se te va la plata." />

      <div className="mb-6 flex flex-wrap gap-2">
        <Select value={scope} onChange={(e) => setScope(e.target.value)} className="sm:w-64">
          <option value="all">Todo (personales + grupos)</option>
          <option value="personal">Sólo gastos personales</option>
          {(groups.data ?? [])
            .filter((group) => !group.archived)
            .map((group) => (
              <option key={group.id} value={group.id}>
                {group.emoji} {group.name}
              </option>
            ))}
        </Select>

        <Select
          value={months}
          onChange={(e) => setMonths(Number(e.target.value))}
          className="sm:w-40"
        >
          <option value={3}>3 meses</option>
          <option value={6}>6 meses</option>
          <option value={12}>12 meses</option>
          <option value={24}>24 meses</option>
        </Select>
      </div>

      {stats.error && <ErrorMessage message={stats.error} />}
      {stats.loading && !stats.data && <Spinner />}

      {stats.data && stats.data.expense_count === 0 && (
        <EmptyState
          title="Nada para mostrar todavía"
          description="Cuando cargues gastos vas a ver acá el desglose por categoría y por mes."
        />
      )}

      {stats.data && stats.data.expense_count > 0 && (
        <div className="space-y-6">
          <div className="grid gap-3 sm:grid-cols-2">
            <Card className="p-5">
              <p className="text-sm text-slate-500 dark:text-slate-400">Total</p>
              <p className="tabular mt-1 text-2xl font-bold">
                {money(stats.data.total_cents, stats.data.currency)}
              </p>
            </Card>
            <Card className="p-5">
              <p className="text-sm text-slate-500 dark:text-slate-400">Gastos registrados</p>
              <p className="tabular mt-1 text-2xl font-bold">{stats.data.expense_count}</p>
            </Card>
          </div>

          <MonthlyChart points={stats.data.by_month} currency={stats.data.currency} />

          <section>
            <h2 className="mb-3 text-lg font-semibold">Por categoría</h2>
            <ul className="space-y-2">
              {stats.data.by_category.map((row) => {
                const share = stats.data!.total_cents
                  ? (row.total_cents / stats.data!.total_cents) * 100
                  : 0
                return (
                  <li key={row.category.id}>
                    <Card className="p-4">
                      <div className="flex items-center gap-3">
                        <span aria-hidden className="text-xl">
                          {row.category.icon}
                        </span>
                        <span className="min-w-0 flex-1 truncate text-sm font-medium">
                          {row.category.name}
                        </span>
                        <span className="text-xs text-slate-500 dark:text-slate-400">
                          {share.toFixed(0)}%
                        </span>
                        <span className="tabular w-28 text-right font-semibold">
                          {money(row.total_cents, stats.data!.currency)}
                        </span>
                      </div>
                      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-slate-100 dark:bg-slate-800">
                        <div
                          className="h-full rounded-full bg-brand-500"
                          style={{ width: `${Math.max(share, 1)}%` }}
                        />
                      </div>
                    </Card>
                  </li>
                )
              })}
            </ul>
          </section>
        </div>
      )}
    </>
  )
}

/** Barras verticales simples: evita sumar una librería de gráficos entera. */
function MonthlyChart({
  points,
  currency,
}: {
  points: { month: string; total_cents: number }[]
  currency: string
}) {
  if (points.length === 0) return null

  const max = Math.max(...points.map((p) => p.total_cents), 1)

  return (
    <section>
      <h2 className="mb-3 text-lg font-semibold">Mes a mes</h2>
      <Card className="p-5">
        <div className="flex h-44 items-end gap-2">
          {points.map((point) => {
            const height = (point.total_cents / max) * 100
            return (
              <div key={point.month} className="group flex min-w-0 flex-1 flex-col items-center gap-2">
                <span className="tabular text-[10px] text-slate-500 opacity-0 transition group-hover:opacity-100 dark:text-slate-400">
                  {money(point.total_cents, currency)}
                </span>
                <div
                  className={cn(
                    'w-full rounded-t-lg bg-brand-500 transition-all',
                    'hover:bg-brand-600',
                  )}
                  style={{ height: `${Math.max(height, 2)}%` }}
                  title={`${formatMonth(point.month)}: ${money(point.total_cents, currency)}`}
                />
                <span className="truncate text-[10px] text-slate-500 dark:text-slate-400">
                  {formatMonth(point.month)}
                </span>
              </div>
            )
          })}
        </div>
      </Card>
    </section>
  )
}
