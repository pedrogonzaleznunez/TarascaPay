// Formateo de dinero y fechas. Todo el dinero se maneja en centavos enteros.

const LOCALE = 'es-AR'

/** Convierte centavos a texto con símbolo de moneda: 123456 → "$ 1.234,56". */
export function money(cents: number, currency = 'ARS'): string {
  try {
    return new Intl.NumberFormat(LOCALE, {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
    }).format(cents / 100)
  } catch {
    // Un código de moneda desconocido no debe romper la pantalla.
    return `${currency} ${(cents / 100).toFixed(2)}`
  }
}

/** Igual que `money` pero siempre en positivo — el signo lo da el contexto. */
export function absMoney(cents: number, currency = 'ARS'): string {
  return money(Math.abs(cents), currency)
}

/** Convierte lo que se escribe en un input ("1.234,56" o "1234.56") a centavos. */
export function parseAmount(input: string): number | null {
  const raw = input.trim()
  if (!raw) return null

  // Se acepta coma o punto como separador decimal: el último separador manda.
  const lastComma = raw.lastIndexOf(',')
  const lastDot = raw.lastIndexOf('.')
  const decimalSep = lastComma > lastDot ? ',' : lastDot > lastComma ? '.' : ''

  let normalized = raw
  if (decimalSep) {
    const [whole, fraction] = [
      raw.slice(0, raw.lastIndexOf(decimalSep)),
      raw.slice(raw.lastIndexOf(decimalSep) + 1),
    ]
    normalized = `${whole.replace(/[^\d-]/g, '')}.${fraction.replace(/\D/g, '')}`
  } else {
    normalized = raw.replace(/[^\d-]/g, '')
  }

  const value = Number(normalized)
  if (!Number.isFinite(value)) return null

  // Redondeo al centavo: evita 0.1+0.2 y compañía.
  return Math.round(value * 100)
}

/** Centavos → texto editable para un input ("1234.56"). */
export function centsToInput(cents: number): string {
  return (cents / 100).toFixed(2)
}

const DATE_FMT = new Intl.DateTimeFormat(LOCALE, {
  day: 'numeric',
  month: 'short',
  year: 'numeric',
})

const DATETIME_FMT = new Intl.DateTimeFormat(LOCALE, {
  day: 'numeric',
  month: 'short',
  hour: '2-digit',
  minute: '2-digit',
})

/** Fecha `YYYY-MM-DD` en texto legible, sin corrimientos por zona horaria. */
export function formatDate(isoDate: string): string {
  const [y, m, d] = isoDate.split('T')[0].split('-').map(Number)
  if (!y || !m || !d) return isoDate
  return DATE_FMT.format(new Date(y, m - 1, d))
}

export function formatDateTime(iso: string): string {
  return DATETIME_FMT.format(new Date(iso))
}

/** "hace 5 min", "ayer", "hace 3 días"… */
export function relativeTime(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime()
  const minutes = Math.round(diffMs / 60000)

  if (minutes < 1) return 'recién'
  if (minutes < 60) return `hace ${minutes} min`

  const hours = Math.round(minutes / 60)
  if (hours < 24) return `hace ${hours} h`

  const days = Math.round(hours / 24)
  if (days === 1) return 'ayer'
  if (days < 30) return `hace ${days} días`

  return formatDate(iso)
}

/** Fecha de hoy en `YYYY-MM-DD` según el huso local (no UTC). */
export function today(): string {
  const now = new Date()
  const offset = now.getTimezoneOffset() * 60000
  return new Date(now.getTime() - offset).toISOString().slice(0, 10)
}

/** "2026-03" → "mar 2026" */
export function formatMonth(month: string): string {
  const [y, m] = month.split('-').map(Number)
  if (!y || !m) return month
  return new Intl.DateTimeFormat(LOCALE, { month: 'short', year: '2-digit' }).format(
    new Date(y, m - 1, 1),
  )
}

/** Iniciales para el avatar: "Ana María" → "AM". */
export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean)
  if (parts.length === 0) return '?'
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase()
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase()
}

/** Color estable derivado del id, para que cada persona tenga siempre el mismo. */
export function avatarColor(id: string): string {
  const palette = [
    'bg-emerald-500',
    'bg-sky-500',
    'bg-violet-500',
    'bg-amber-500',
    'bg-rose-500',
    'bg-teal-500',
    'bg-indigo-500',
    'bg-orange-500',
  ]
  let hash = 0
  for (let i = 0; i < id.length; i++) hash = (hash * 31 + id.charCodeAt(i)) >>> 0
  return palette[hash % palette.length]
}

export function fileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} kB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
