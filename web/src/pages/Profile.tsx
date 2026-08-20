// Perfil: datos de la cuenta, tema y cambio de contraseña.

import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { LogOut, Monitor, Moon, Sun } from 'lucide-react'

import { api } from '../lib/api'
import { useAuth } from '../lib/auth'
import { formatDate } from '../lib/format'
import { errorMessage } from '../lib/hooks'
import { useTheme } from '../lib/theme'
import type { Theme } from '../lib/theme'
import { useToast } from '../lib/toast'
import { Avatar, Button, Card, ErrorMessage, Field, Input, PageTitle, Select } from '../components/ui'

const CURRENCIES = ['ARS', 'USD', 'EUR', 'BRL', 'CLP', 'UYU', 'MXN', 'COP', 'PEN', 'GBP']

const THEMES: { value: Theme; label: string; icon: typeof Sun }[] = [
  { value: 'light', label: 'Claro', icon: Sun },
  { value: 'dark', label: 'Oscuro', icon: Moon },
  { value: 'system', label: 'Sistema', icon: Monitor },
]

export function ProfilePage() {
  const { user, setUser, logout } = useAuth()
  const { theme, setTheme } = useTheme()
  const toast = useToast()
  const navigate = useNavigate()

  const [displayName, setDisplayName] = useState(user?.display_name ?? '')
  const [currency, setCurrency] = useState(user?.currency ?? 'ARS')
  const [savingProfile, setSavingProfile] = useState(false)
  const [profileError, setProfileError] = useState<string | null>(null)

  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [savingPassword, setSavingPassword] = useState(false)
  const [passwordError, setPasswordError] = useState<string | null>(null)

  if (!user) return null

  async function saveProfile(event: React.FormEvent) {
    event.preventDefault()
    setProfileError(null)
    setSavingProfile(true)
    try {
      const updated = await api.updateProfile({ display_name: displayName, currency })
      setUser(updated)
      toast.success('Perfil actualizado')
    } catch (err) {
      setProfileError(errorMessage(err))
    } finally {
      setSavingProfile(false)
    }
  }

  async function savePassword(event: React.FormEvent) {
    event.preventDefault()
    setPasswordError(null)
    setSavingPassword(true)
    try {
      await api.changePassword({
        current_password: currentPassword,
        new_password: newPassword,
      })
      toast.success('Contraseña actualizada')
      setCurrentPassword('')
      setNewPassword('')
    } catch (err) {
      setPasswordError(errorMessage(err))
    } finally {
      setSavingPassword(false)
    }
  }

  return (
    <>
      <PageTitle title="Perfil" subtitle="Tu cuenta y las preferencias de la app." />

      <div className="space-y-5">
        <Card className="flex items-center gap-4 p-5">
          <Avatar user={user} size="lg" />
          <div className="min-w-0">
            <p className="truncate text-lg font-semibold">{user.display_name}</p>
            <p className="truncate text-sm text-slate-500 dark:text-slate-400">{user.email}</p>
            <p className="mt-0.5 text-xs text-slate-400">
              En TarascaPay desde {formatDate(user.created_at)}
            </p>
          </div>
        </Card>

        <Card className="p-5">
          <h2 className="mb-4 font-semibold">Datos</h2>
          <form onSubmit={saveProfile} className="space-y-4">
            {profileError && <ErrorMessage message={profileError} />}

            <Field label="Nombre">
              <Input
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                maxLength={80}
                required
              />
            </Field>

            <Field
              label="Moneda principal"
              hint="Se usa en tus gastos personales y en el panel."
            >
              <Select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                {CURRENCIES.map((code) => (
                  <option key={code} value={code}>
                    {code}
                  </option>
                ))}
              </Select>
            </Field>

            <Button type="submit" loading={savingProfile}>
              Guardar
            </Button>
          </form>
        </Card>

        <Card className="p-5">
          <h2 className="mb-4 font-semibold">Apariencia</h2>
          <div className="grid grid-cols-3 gap-2">
            {THEMES.map(({ value, label, icon: Icon }) => (
              <button
                key={value}
                type="button"
                onClick={() => setTheme(value)}
                className={`flex flex-col items-center gap-2 rounded-xl px-3 py-4 text-sm font-medium transition ${
                  theme === value
                    ? 'bg-brand-600 text-white'
                    : 'bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700'
                }`}
              >
                <Icon className="size-5" />
                {label}
              </button>
            ))}
          </div>
        </Card>

        <Card className="p-5">
          <h2 className="mb-4 font-semibold">Cambiar contraseña</h2>
          <form onSubmit={savePassword} className="space-y-4">
            {passwordError && <ErrorMessage message={passwordError} />}

            <Field label="Contraseña actual">
              <Input
                type="password"
                value={currentPassword}
                onChange={(e) => setCurrentPassword(e.target.value)}
                autoComplete="current-password"
                required
              />
            </Field>

            <Field label="Nueva contraseña" hint="Mínimo 8 caracteres">
              <Input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                autoComplete="new-password"
                minLength={8}
                required
              />
            </Field>

            <Button type="submit" loading={savingPassword}>
              Cambiar contraseña
            </Button>
          </form>
        </Card>

        <Button
          variant="secondary"
          className="w-full"
          icon={<LogOut className="size-4" />}
          onClick={() => {
            logout()
            navigate('/ingresar')
          }}
        >
          Cerrar sesión
        </Button>
      </div>
    </>
  )
}
