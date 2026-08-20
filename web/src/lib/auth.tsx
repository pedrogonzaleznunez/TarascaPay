// Sesión del usuario: carga inicial, login/registro y cierre de sesión.

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'

import { api, getToken, onUnauthorized, setToken } from './api'
import type { User } from './types'

interface AuthContextValue {
  user: User | null
  /** `true` mientras se revalida el token guardado al abrir la app. */
  loading: boolean
  login: (email: string, password: string) => Promise<void>
  register: (data: {
    email: string
    password: string
    display_name: string
    currency?: string
  }) => Promise<void>
  logout: () => void
  /** Refresca los datos del usuario tras editar el perfil. */
  setUser: (user: User) => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUserState] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)

  const logout = useCallback(() => {
    setToken(null)
    setUserState(null)
  }, [])

  // Un 401 en cualquier request cierra la sesión desde un solo lugar.
  useEffect(() => {
    onUnauthorized.handler = () => setUserState(null)
    return () => {
      onUnauthorized.handler = null
    }
  }, [])

  // Al abrir la app se valida el token guardado contra el servidor.
  useEffect(() => {
    let cancelled = false

    if (!getToken()) {
      setLoading(false)
      return
    }

    api
      .me()
      .then((me) => {
        if (!cancelled) setUserState(me)
      })
      .catch(() => {
        if (!cancelled) setToken(null)
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [])

  const login = useCallback(async (email: string, password: string) => {
    const result = await api.login({ email, password })
    setToken(result.token)
    setUserState(result.user)
  }, [])

  const register = useCallback(
    async (data: { email: string; password: string; display_name: string; currency?: string }) => {
      const result = await api.register(data)
      setToken(result.token)
      setUserState(result.user)
    },
    [],
  )

  const value = useMemo(
    () => ({ user, loading, login, register, logout, setUser: setUserState }),
    [user, loading, login, register, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext)
  if (!context) throw new Error('useAuth debe usarse dentro de <AuthProvider>')
  return context
}
