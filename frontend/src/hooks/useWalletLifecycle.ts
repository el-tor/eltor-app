import { useEffect } from 'react'
import { useDispatch } from '../hooks'
import { refreshWallet } from '../features/wallet/walletStore'
import { isTauri } from '../utils/platform'

// Mounted above routes: navigation is never required to discover wallet readiness.
export function useWalletLifecycle() {
  const dispatch = useDispatch()
  useEffect(() => {
    let disposed = false
    let running = false
    let queued = false
    let timer: ReturnType<typeof setTimeout> | undefined
    let listeners: Array<() => void> = []
    let request: { abort: () => void } | undefined
    const refresh = async () => {
      if (disposed) return
      clearTimeout(timer)
      if (running) { queued = true; return }
      running = true
      const current = dispatch(refreshWallet())
      request = current
      const result = await current
      running = false
      if (disposed) return
      if (queued) { queued = false; void refresh(); return }
      const ready = refreshWallet.fulfilled.match(result) && !!result.payload.info
      timer = setTimeout(refresh, ready ? 15000 : 2000)
    }
    const initialize = async () => {
      if (isTauri()) {
        try {
          const { listen } = await import('@tauri-apps/api/event')
          for (const event of ['phoenixd-ready', 'lightning-ready', 'phoenixd-stopped', 'phoenixd-error']) {
            const unlisten = await listen(event, refresh)
            if (disposed) unlisten()
            else listeners.push(unlisten)
          }
        } catch { /* Polling also recovers events emitted before UI listeners. */ }
      }
      if (!disposed) void refresh()
    }
    void initialize()
    return () => {
      disposed = true
      clearTimeout(timer)
      listeners.forEach(unlisten => unlisten())
      // Suppress completion from the old StrictMode mount after a new one starts.
      request?.abort()
    }
  }, [dispatch])
}
