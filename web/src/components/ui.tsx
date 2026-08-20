// Piezas de interfaz reutilizables.

import { useEffect, useRef } from 'react'
import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode, SelectHTMLAttributes, TextareaHTMLAttributes } from 'react'
import { Loader2, X } from 'lucide-react'

import { avatarColor, initials } from '../lib/format'
import type { UserBrief } from '../lib/types'

export function cn(...classes: (string | false | null | undefined)[]): string {
  return classes.filter(Boolean).join(' ')
}

// ---------------------------------------------------------------------------
// Botones
// ---------------------------------------------------------------------------

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant
  loading?: boolean
  size?: 'sm' | 'md'
  icon?: ReactNode
}

const VARIANTS: Record<ButtonVariant, string> = {
  primary: 'bg-brand-600 text-white hover:bg-brand-700 disabled:bg-brand-600/50',
  secondary:
    'bg-white text-slate-700 ring-1 ring-slate-300 hover:bg-slate-50 dark:bg-slate-800 dark:text-slate-200 dark:ring-slate-700 dark:hover:bg-slate-700',
  ghost:
    'text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800',
  danger: 'bg-rose-600 text-white hover:bg-rose-700 disabled:bg-rose-600/50',
}

export function Button({
  variant = 'primary',
  loading = false,
  size = 'md',
  icon,
  className,
  children,
  disabled,
  ...rest
}: ButtonProps) {
  return (
    <button
      {...rest}
      disabled={disabled || loading}
      className={cn(
        'inline-flex items-center justify-center gap-2 rounded-xl font-medium transition',
        'disabled:cursor-not-allowed disabled:opacity-70',
        size === 'sm' ? 'px-3 py-1.5 text-sm' : 'px-4 py-2.5 text-sm',
        VARIANTS[variant],
        className,
      )}
    >
      {loading ? <Loader2 className="size-4 animate-spin" aria-hidden /> : icon}
      {children}
    </button>
  )
}

// ---------------------------------------------------------------------------
// Campos de formulario
// ---------------------------------------------------------------------------

interface FieldProps {
  label?: string
  hint?: string
  error?: string
  children: ReactNode
  className?: string
}

export function Field({ label, hint, error, children, className }: FieldProps) {
  return (
    <div className={className}>
      {label && <span className="label">{label}</span>}
      {children}
      {error ? (
        <p className="mt-1 text-xs text-rose-600 dark:text-rose-400">{error}</p>
      ) : hint ? (
        <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">{hint}</p>
      ) : null}
    </div>
  )
}

export function Input({ className, ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...rest} className={cn('field', className)} />
}

export function Textarea({ className, ...rest }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...rest} className={cn('field resize-y', className)} />
}

export function Select({ className, children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select {...rest} className={cn('field cursor-pointer', className)}>
      {children}
    </select>
  )
}

// ---------------------------------------------------------------------------
// Estructura
// ---------------------------------------------------------------------------

export function Card({ className, children }: { className?: string; children: ReactNode }) {
  return <div className={cn('card', className)}>{children}</div>
}

export function PageTitle({
  title,
  subtitle,
  action,
}: {
  title: string
  subtitle?: string
  action?: ReactNode
}) {
  return (
    <div className="mb-6 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 className="text-2xl font-bold tracking-tight sm:text-3xl">{title}</h1>
        {subtitle && <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">{subtitle}</p>}
      </div>
      {action}
    </div>
  )
}

export function Spinner({ className }: { className?: string }) {
  return (
    <div className={cn('flex items-center justify-center py-12', className)}>
      <Loader2 className="size-6 animate-spin text-brand-600" aria-label="Cargando" />
    </div>
  )
}

export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon?: ReactNode
  title: string
  description?: string
  action?: ReactNode
}) {
  return (
    <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-slate-300 px-6 py-14 text-center dark:border-slate-700">
      {icon && <div className="mb-3 text-slate-400 dark:text-slate-500">{icon}</div>}
      <p className="font-semibold">{title}</p>
      {description && (
        <p className="mt-1 max-w-sm text-sm text-slate-500 dark:text-slate-400">{description}</p>
      )}
      {action && <div className="mt-5">{action}</div>}
    </div>
  )
}

export function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-xl bg-rose-50 px-4 py-3 text-sm text-rose-700 ring-1 ring-rose-200 dark:bg-rose-950/50 dark:text-rose-300 dark:ring-rose-900">
      {message}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Modal
// ---------------------------------------------------------------------------

export function Modal({
  open,
  onClose,
  title,
  children,
  wide = false,
}: {
  open: boolean
  onClose: () => void
  title: string
  children: ReactNode
  wide?: boolean
}) {
  const panelRef = useRef<HTMLDivElement>(null)

  // Cerrar con Escape y bloquear el scroll del fondo mientras está abierto.
  useEffect(() => {
    if (!open) return

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onKeyDown)

    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    panelRef.current?.focus()

    return () => {
      document.removeEventListener('keydown', onKeyDown)
      document.body.style.overflow = previousOverflow
    }
  }, [open, onClose])

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-end justify-center overflow-y-auto bg-slate-900/50 p-0 backdrop-blur-sm sm:items-center sm:p-4">
      {/* Clic en el fondo para cerrar. */}
      <div className="absolute inset-0" onClick={onClose} aria-hidden />
      <div
        ref={panelRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cn(
          'animate-rise relative my-0 w-full rounded-t-2xl bg-white shadow-xl outline-none sm:my-8 sm:rounded-2xl dark:bg-slate-900',
          wide ? 'sm:max-w-2xl' : 'sm:max-w-lg',
        )}
      >
        <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4 dark:border-slate-800">
          <h2 className="text-lg font-semibold">{title}</h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Cerrar"
            className="rounded-lg p-1.5 text-slate-500 transition hover:bg-slate-100 dark:hover:bg-slate-800"
          >
            <X className="size-5" />
          </button>
        </div>
        <div className="max-h-[70vh] overflow-y-auto px-5 py-5">{children}</div>
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Avatares
// ---------------------------------------------------------------------------

const AVATAR_SIZES = {
  xs: 'size-6 text-[10px]',
  sm: 'size-8 text-xs',
  md: 'size-10 text-sm',
  lg: 'size-14 text-lg',
}

export function Avatar({
  user,
  size = 'md',
  className,
}: {
  user: Pick<UserBrief, 'id' | 'display_name' | 'avatar_url'>
  size?: keyof typeof AVATAR_SIZES
  className?: string
}) {
  if (user.avatar_url) {
    return (
      <img
        src={user.avatar_url}
        alt={user.display_name}
        className={cn('shrink-0 rounded-full object-cover', AVATAR_SIZES[size], className)}
      />
    )
  }

  return (
    <span
      title={user.display_name}
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded-full font-semibold text-white',
        avatarColor(user.id),
        AVATAR_SIZES[size],
        className,
      )}
    >
      {initials(user.display_name)}
    </span>
  )
}

export function AvatarStack({ users, max = 4 }: { users: UserBrief[]; max?: number }) {
  const shown = users.slice(0, max)
  const rest = users.length - shown.length

  return (
    <div className="flex -space-x-2">
      {shown.map((user) => (
        <Avatar
          key={user.id}
          user={user}
          size="sm"
          className="ring-2 ring-white dark:ring-slate-900"
        />
      ))}
      {rest > 0 && (
        <span className="inline-flex size-8 items-center justify-center rounded-full bg-slate-200 text-xs font-semibold text-slate-600 ring-2 ring-white dark:bg-slate-700 dark:text-slate-300 dark:ring-slate-900">
          +{rest}
        </span>
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Varios
// ---------------------------------------------------------------------------

export function Badge({
  children,
  tone = 'neutral',
}: {
  children: ReactNode
  tone?: 'neutral' | 'positive' | 'negative' | 'brand'
}) {
  const tones = {
    neutral: 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300',
    positive: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300',
    negative: 'bg-rose-100 text-rose-700 dark:bg-rose-950 dark:text-rose-300',
    brand: 'bg-brand-100 text-brand-800 dark:bg-brand-900/60 dark:text-brand-200',
  }
  return (
    <span
      className={cn('rounded-full px-2 py-0.5 text-xs font-medium whitespace-nowrap', tones[tone])}
    >
      {children}
    </span>
  )
}

/** Importe con color según el signo: verde te deben, rojo debés. */
export function SignedAmount({
  cents,
  currency,
  className,
}: {
  cents: number
  currency: string
  className?: string
}) {
  const tone =
    cents > 0
      ? 'text-emerald-600 dark:text-emerald-400'
      : cents < 0
        ? 'text-rose-600 dark:text-rose-400'
        : 'text-slate-500 dark:text-slate-400'

  return (
    <span className={cn('tabular font-semibold', tone, className)}>
      {formatSigned(cents, currency)}
    </span>
  )
}

function formatSigned(cents: number, currency: string): string {
  const formatted = new Intl.NumberFormat('es-AR', {
    style: 'currency',
    currency: currency || 'ARS',
    minimumFractionDigits: 2,
  }).format(Math.abs(cents) / 100)
  if (cents === 0) return formatted
  return `${cents > 0 ? '+' : '−'}${formatted}`
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = 'Confirmar',
  danger = false,
  loading = false,
  onConfirm,
  onCancel,
}: {
  open: boolean
  title: string
  message: string
  confirmLabel?: string
  danger?: boolean
  loading?: boolean
  onConfirm: () => void
  onCancel: () => void
}) {
  return (
    <Modal open={open} onClose={onCancel} title={title}>
      <p className="text-sm text-slate-600 dark:text-slate-300">{message}</p>
      <div className="mt-6 flex justify-end gap-2">
        <Button variant="secondary" onClick={onCancel}>
          Cancelar
        </Button>
        <Button variant={danger ? 'danger' : 'primary'} loading={loading} onClick={onConfirm}>
          {confirmLabel}
        </Button>
      </div>
    </Modal>
  )
}
