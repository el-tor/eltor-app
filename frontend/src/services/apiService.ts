import { isTauri } from '../utils/platform'

// Tauri imports (only loaded when in Tauri mode)
let tauriInvoke: any = null
let tauriListen: any = null

// Load Tauri APIs dynamically
const loadTauriAPIs = async () => {
  if (isTauri() && !tauriInvoke) {
    const coreModule = await import('@tauri-apps/api/core')
    const eventModule = await import('@tauri-apps/api/event')
    tauriInvoke = coreModule.invoke
    tauriListen = eventModule.listen
  }
}

// Web API base URL
const WEB_API_BASE = 'http://localhost:8080'

export interface EltordStatus {
  running: boolean
  pid?: number
  recent_logs?: LogEntry[]
}

export interface LogEntry {
  timestamp: string
  level: string
  message: string
  source: string
}

export interface TorStatus {
  connected: boolean
  circuit?: string
}

class ApiService {
  // Eltord methods
  async activateEltord(): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
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
      await loadTauriAPIs()
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
      await loadTauriAPIs()
      return await tauriInvoke('get_eltord_status')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/eltord/status`)
      return await response.json()
    }
  }

  // Tor methods
  async connectTor(): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
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
      await loadTauriAPIs()
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
      await loadTauriAPIs()
      return await tauriInvoke('get_tor_status')
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/tor/status`)
      return await response.json()
    }
  }

  // Test method to verify event system
  async testLogEvent(): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('test_log_event')
    } else {
      return 'Test log event not available in web mode'
    }
  }

  // Event listening (only works in Tauri)
  async listenToEvents(callback: (eventName: string, payload: any) => void) {
    if (isTauri()) {
      await loadTauriAPIs()
      
      if (!tauriListen) {
        console.error('Tauri listen API not available')
        return () => {}
      }

      console.log('Setting up Tauri event listeners...')
      
      const unlistenActivated = await tauriListen(
        'eltord-activated',
        (event: any) => {
          console.log('Received eltord-activated event:', event)
          callback('eltord-activated', event.payload)
        },
      )

      const unlistenDeactivated = await tauriListen(
        'eltord-deactivated',
        (event: any) => {
          console.log('Received eltord-deactivated event:', event)
          callback('eltord-deactivated', event.payload)
        },
      )

      const unlistenError = await tauriListen('eltord-error', (event: any) => {
        console.log('Received eltord-error event:', event)
        callback('eltord-error', event.payload)
      })

      const unlistenLog = await tauriListen('eltord-log', (event: any) => {
        console.log('Received eltord-log event:', event)
        callback('eltord-log', event.payload)
      })

      console.log('All Tauri event listeners set up successfully')

      return () => {
        console.log('Cleaning up Tauri event listeners')
        unlistenActivated()
        unlistenDeactivated()
        unlistenError()
        unlistenLog()
      }
    }

    // For web mode, return empty cleanup function
    return () => {}
  }

  // Log streaming for web mode (Server-Sent Events)
  createLogStream(onLog: (log: LogEntry) => void, onError?: (error: Error) => void): () => void {
    if (isTauri()) {
      // In Tauri mode, logs come through events
      console.warn('Log streaming not available in Tauri mode - use listenToEvents instead')
      return () => {}
    }

    const eventSource = new EventSource(`${WEB_API_BASE}/api/eltord/logs`)
    
    eventSource.onmessage = (event) => {
      try {
        const logEntry: LogEntry = JSON.parse(event.data)
        onLog(logEntry)
      } catch (error) {
        console.error('Failed to parse log entry:', error)
        onError?.(new Error('Failed to parse log entry'))
      }
    }

    eventSource.onerror = (error) => {
      console.error('SSE error:', error)
      onError?.(new Error('Log stream connection error'))
    }

    // Return cleanup function
    return () => {
      eventSource.close()
    }
  }
}

export const apiService = new ApiService()
