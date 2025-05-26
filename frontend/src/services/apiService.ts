import { isTauri } from '../utils/platform'

// Tauri imports (only loaded when in Tauri mode)
let tauriInvoke: any = null
let tauriListen: any = null

if (isTauri()) {
  import('@tauri-apps/api/core').then((module) => {
    tauriInvoke = module.invoke
  })
  import('@tauri-apps/api/event').then((module) => {
    tauriListen = module.listen
  })
}

// Web API base URL
const WEB_API_BASE = 'http://localhost:8080'

export interface EltordStatus {
  running: boolean
  pid?: number
}

export interface TorStatus {
  connected: boolean
  circuit?: string
}

class ApiService {
  // Eltord methods
  async activateEltord(): Promise<string> {
    if (isTauri()) {
      return await tauriInvoke('activate_eltord')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/eltord/activate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      })

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      const data = await response.json()
      return data.message
    }
  }

  async deactivateEltord(): Promise<string> {
    if (isTauri()) {
      return await tauriInvoke('deactivate_eltord')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/eltord/deactivate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      })

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      const data = await response.json()
      return data.message
    }
  }

  async getEltordStatus(): Promise<EltordStatus> {
    if (isTauri()) {
      return await tauriInvoke('get_eltord_status')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/eltord/status`)
      return await response.json()
    }
  }

  // Tor methods
  async connectTor(): Promise<string> {
    if (isTauri()) {
      return await tauriInvoke('connect_tor')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/tor/connect`, {
        method: 'POST',
      })
      const data = await response.json()
      return data.message || 'Connected'
    }
  }

  async disconnectTor(): Promise<string> {
    if (isTauri()) {
      return await tauriInvoke('disconnect_tor')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/tor/disconnect`, {
        method: 'POST',
      })
      const data = await response.json()
      return data.message || 'Disconnected'
    }
  }

  async getTorStatus(): Promise<TorStatus> {
    if (isTauri()) {
      return await tauriInvoke('get_tor_status')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/tor/status`)
      return await response.json()
    }
  }

  // Event listening (only works in Tauri)
  async listenToEvents(callback: (eventName: string, payload: any) => void) {
    if (isTauri() && tauriListen) {
      const unlistenActivated = await tauriListen(
        'eltord-activated',
        (event: any) => {
          callback('eltord-activated', event.payload)
        },
      )

      const unlistenDeactivated = await tauriListen(
        'eltord-deactivated',
        (event: any) => {
          callback('eltord-deactivated', event.payload)
        },
      )

      const unlistenError = await tauriListen('eltord-error', (event: any) => {
        callback('eltord-error', event.payload)
      })

      return () => {
        unlistenActivated()
        unlistenDeactivated()
        unlistenError()
      }
    }

    // For web mode, return empty cleanup function
    return () => {}
  }
}

export const apiService = new ApiService()
