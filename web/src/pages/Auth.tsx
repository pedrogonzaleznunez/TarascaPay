// Ingreso y creación de cuenta.

import { useState } from 'react'
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom'

import { useAuth } from '../lib/auth'
import { errorMessage } from '../lib/hooks'
import { Button, ErrorMessage, Field, Input, Select } from '../components/ui'

const CURRENCIES = ['ARS', 'USD', 'EUR', 'BRL', 'CLP', 'UYU', 'MXN', 'COP', 'PEN', 'GBP']

export function AuthPage({ mode }: { mode: 'login' | 'register' }) {
  const { user, loading, login, register } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()

  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [currency, setCurrency] = useState('ARS')
  const [error, setError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  if (!loading && user) {
    const from = (location.state as { from?: string } | null)?.from ?? '/'
    return <Navigate to={from} replace />
  }

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault()
    setError(null)
    setSubmitting(true)
    try {
      if (mode === 'login') {
        await login(email, password)
      } else {
        await register({ email, password, display_name: displayName, currency })
      }
      navigate('/', { replace: true })
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="flex min-h-full items-center justify-center px-4 py-10">
      <div className="w-full max-w-md">
        <div className="mb-8 flex flex-col items-center text-center">
          <img src="/tarasca.svg" alt="" className="mb-4 size-16 rounded-2xl shadow-lg" />
          <h1 className="text-3xl font-bold tracking-tight">TarascaPay</h1>
          <p className="mt-2 text-sm text-slate-500 dark:text-slate-400">
            Dividí gastos de viajes y del día a día, sin planillas ni discusiones.
          </p>
        </div>

        <div className="card p-6">
          <h2 className="mb-5 text-lg font-semibold">
            {mode === 'login' ? 'Entrar a tu cuenta' : 'Crear una cuenta'}
          </h2>

          <form onSubmit={handleSubmit} className="space-y-4">
            {error && <ErrorMessage message={error} />}

            {mode === 'register' && (
              <Field label="Cómo te llamás">
                <Input
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  placeholder="Ana Pérez"
                  autoComplete="name"
                  required
                  maxLength={80}
                />
              </Field>
            )}

            <Field label="Email">
              <Input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="vos@ejemplo.com"
                autoComplete="email"
                required
              />
            </Field>

            <Field
              label="Contraseña"
              hint={mode === 'register' ? 'Mínimo 8 caracteres' : undefined}
            >
              <Input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete={mode === 'login' ? 'current-password' : 'new-password'}
                required
                minLength={mode === 'register' ? 8 : undefined}
              />
            </Field>

            {mode === 'register' && (
              <Field label="Moneda principal">
                <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                  {CURRENCIES.map((code) => (
                    <option key={code} value={code}>
                      {code}
                    </option>
                  ))}
                </Select>
              </Field>
            )}

            <Button type="submit" loading={submitting} className="w-full">
              {mode === 'login' ? 'Entrar' : 'Crear cuenta'}
            </Button>
          </form>

          <p className="mt-5 text-center text-sm text-slate-500 dark:text-slate-400">
            {mode === 'login' ? (
              <>
                ¿No tenés cuenta?{' '}
                <Link to="/registro" className="font-medium text-brand-600 hover:underline">
                  Registrate
                </Link>
              </>
            ) : (
              <>
                ¿Ya tenés cuenta?{' '}
                <Link to="/ingresar" className="font-medium text-brand-600 hover:underline">
                  Entrá
                </Link>
              </>
            )}
          </p>
        </div>
      </div>
    </div>
  )
}
