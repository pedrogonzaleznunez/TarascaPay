// Hooks de datos compartidos por las pantallas.

import { useCallback, useEffect, useRef, useState } from 'react'

import { ApiError } from './api'

/** Evento global para que las pantallas recarguen tras crear algo desde el shell. */
export const REFRESH_EVENT = 'tarascapay:refresh'

export function emitRefresh() {
  window.dispatchEvent(new CustomEvent(REFRESH_EVENT))
}

interface AsyncState<T> {
  data: T | null
  loading: boolean
  error: string | null
  reload: () => void
}

/**
 * Ejecuta `loader` al montar y cada vez que cambian las `deps`, o cuando algo
 * emite el evento global de refresco.
 *
 * Descarta las respuestas de peticiones que quedaron obsoletas, así una
 * respuesta lenta no pisa a una más nueva.
 */
export function useAsync<T>(loader: () => Promise<T>, deps: unknown[]): AsyncState<T> {
  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [nonce, setNonce] = useState(0)

  // `loader` suele ser una función nueva en cada render; la guardamos en una
  // ref para que el efecto dependa sólo de `deps`.
  const loaderRef = useRef(loader)
  loaderRef.current = loader

  const requestId = useRef(0)

  useEffect(() => {
    const id = ++requestId.current
    setLoading(true)
    setError(null)

    loaderRef
      .current()
      .then((result) => {
        if (id !== requestId.current) return
        setData(result)
      })
      .catch((err: unknown) => {
        if (id !== requestId.current) return
        setError(err instanceof ApiError ? err.message : 'Ocurrió un error inesperado')
      })
      .finally(() => {
        if (id === requestId.current) setLoading(false)
      })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, nonce])

  const reload = useCallback(() => setNonce((n) => n + 1), [])

  useEffect(() => {
    const listener = () => reload()
    window.addEventListener(REFRESH_EVENT, listener)
    return () => window.removeEventListener(REFRESH_EVENT, listener)
  }, [reload])

  return { data, loading, error, reload }
}

/** Mensaje legible a partir de cualquier error capturado. */
export function errorMessage(err: unknown): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return 'Ocurrió un error inesperado'
}
