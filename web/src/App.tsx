// Rutas de la SPA.

import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom'
import type { ReactNode } from 'react'

import { AuthProvider, useAuth } from './lib/auth'
import { ThemeProvider } from './lib/theme'
import { ToastProvider } from './lib/toast'
import { Layout } from './components/Layout'
import { Spinner } from './components/ui'
import { AuthPage } from './pages/Auth'
import { DashboardPage } from './pages/Dashboard'
import { GroupDetailPage } from './pages/GroupDetail'
import { GroupsPage } from './pages/Groups'
import { PersonalPage } from './pages/Personal'
import { ProfilePage } from './pages/Profile'
import { StatsPage } from './pages/Stats'

export default function App() {
  return (
    <ThemeProvider>
      <BrowserRouter>
        <AuthProvider>
          <ToastProvider>
            <Routes>
              <Route path="/ingresar" element={<AuthPage mode="login" />} />
              <Route path="/registro" element={<AuthPage mode="register" />} />

              <Route
                element={
                  <RequireAuth>
                    <Layout />
                  </RequireAuth>
                }
              >
                <Route index element={<DashboardPage />} />
                <Route path="grupos" element={<GroupsPage />} />
                <Route path="grupos/:groupId" element={<GroupDetailPage />} />
                <Route path="personal" element={<PersonalPage />} />
                <Route path="resumen" element={<StatsPage />} />
                <Route path="perfil" element={<ProfilePage />} />
              </Route>

              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </ToastProvider>
        </AuthProvider>
      </BrowserRouter>
    </ThemeProvider>
  )
}

/** Bloquea las rutas privadas y recuerda a dónde quería ir el usuario. */
function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth()
  const location = useLocation()

  if (loading) {
    return (
      <div className="flex min-h-full items-center justify-center">
        <Spinner />
      </div>
    )
  }

  if (!user) {
    return <Navigate to="/ingresar" state={{ from: location.pathname }} replace />
  }

  return <>{children}</>
}
