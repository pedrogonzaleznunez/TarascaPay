// Estructura de navegación: barra lateral en escritorio, barra inferior en móvil.

import { NavLink, Outlet, useNavigate } from 'react-router-dom'
import { BarChart3, Home, LogOut, Plus, User as UserIcon, Users, Wallet } from 'lucide-react'
import { useState } from 'react'

import { useAuth } from '../lib/auth'
import { Avatar, Button, cn } from './ui'
import { ExpenseFormModal } from './ExpenseForm'

const NAV = [
  { to: '/', label: 'Inicio', icon: Home, end: true },
  { to: '/grupos', label: 'Grupos', icon: Users, end: false },
  { to: '/personal', label: 'Personal', icon: Wallet, end: false },
  { to: '/resumen', label: 'Resumen', icon: BarChart3, end: false },
  { to: '/perfil', label: 'Perfil', icon: UserIcon, end: false },
]

export function Layout() {
  const { user, logout } = useAuth()
  const navigate = useNavigate()
  const [creating, setCreating] = useState(false)

  if (!user) return null

  return (
    <div className="min-h-full">
      {/* ---------- Barra lateral (escritorio) ---------- */}
      <aside className="fixed inset-y-0 left-0 hidden w-64 flex-col border-r border-slate-200 bg-white px-4 py-6 lg:flex dark:border-slate-800 dark:bg-slate-900">
        <div className="mb-8 flex items-center gap-2.5 px-2">
          <img src="/tarasca.svg" alt="" className="size-9 rounded-xl" />
          <span className="text-lg font-bold tracking-tight">TarascaPay</span>
        </div>

        <Button icon={<Plus className="size-4" />} onClick={() => setCreating(true)} className="mb-6">
          Nuevo gasto
        </Button>

        <nav className="flex flex-1 flex-col gap-1">
          {NAV.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                cn(
                  'flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition',
                  isActive
                    ? 'bg-brand-50 text-brand-700 dark:bg-brand-900/40 dark:text-brand-300'
                    : 'text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800',
                )
              }
            >
              <Icon className="size-5" aria-hidden />
              {label}
            </NavLink>
          ))}
        </nav>

        <div className="mt-4 flex items-center gap-3 rounded-xl px-2 py-2">
          <Avatar user={user} size="sm" />
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-medium">{user.display_name}</p>
            <p className="truncate text-xs text-slate-500 dark:text-slate-400">{user.email}</p>
          </div>
          <button
            type="button"
            onClick={() => {
              logout()
              navigate('/ingresar')
            }}
            aria-label="Cerrar sesión"
            className="rounded-lg p-1.5 text-slate-500 transition hover:bg-slate-100 dark:hover:bg-slate-800"
          >
            <LogOut className="size-4" />
          </button>
        </div>
      </aside>

      {/* ---------- Cabecera (móvil) ---------- */}
      <header className="sticky top-0 z-30 flex items-center justify-between border-b border-slate-200 bg-white/90 px-4 py-3 backdrop-blur lg:hidden dark:border-slate-800 dark:bg-slate-900/90">
        <div className="flex items-center gap-2">
          <img src="/tarasca.svg" alt="" className="size-8 rounded-lg" />
          <span className="font-bold tracking-tight">TarascaPay</span>
        </div>
        <Avatar user={user} size="sm" />
      </header>

      {/* ---------- Contenido ---------- */}
      <main className="px-4 pt-6 pb-28 lg:ml-64 lg:px-8 lg:pb-10">
        <div className="mx-auto w-full max-w-5xl">
          <Outlet />
        </div>
      </main>

      {/* ---------- Barra inferior (móvil) ---------- */}
      <nav className="fixed inset-x-0 bottom-0 z-30 border-t border-slate-200 bg-white/95 pb-[env(safe-area-inset-bottom)] backdrop-blur lg:hidden dark:border-slate-800 dark:bg-slate-900/95">
        <div className="grid grid-cols-5">
          {NAV.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                cn(
                  'flex flex-col items-center gap-1 py-2.5 text-[11px] font-medium transition',
                  isActive
                    ? 'text-brand-600 dark:text-brand-400'
                    : 'text-slate-500 dark:text-slate-400',
                )
              }
            >
              <Icon className="size-5" aria-hidden />
              {label}
            </NavLink>
          ))}
        </div>
      </nav>

      {/* Botón flotante para cargar un gasto desde cualquier pantalla. */}
      <button
        type="button"
        onClick={() => setCreating(true)}
        aria-label="Nuevo gasto"
        className="fixed right-5 bottom-20 z-30 flex size-14 items-center justify-center rounded-full bg-brand-600 text-white shadow-lg transition hover:bg-brand-700 lg:hidden"
      >
        <Plus className="size-6" />
      </button>

      <ExpenseFormModal
        open={creating}
        onClose={() => setCreating(false)}
        onSaved={() => {
          setCreating(false)
          // La pantalla actual se recarga sola vía su propio efecto de datos.
          window.dispatchEvent(new CustomEvent('tarascapay:refresh'))
        }}
      />
    </div>
  )
}
