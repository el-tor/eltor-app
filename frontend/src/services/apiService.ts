import { isTauri } from '../utils/platform'
import config from '../config'

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

// Web API base URL - construct from current location
const getApiBaseUrl = () => {
  // Just use the current location's protocol, hostname, and port
  // This works because the frontend is served from the same server as the API
  const currentUrl = `${window.location.protocol}//${window.location.host}`
  console.log('apiService - Using current location as API base:', currentUrl)
  return currentUrl
}

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
  mode?: string // "client", "relay", or undefined for system logs
}

export interface TorStatus {
  connected: boolean
  circuit?: string
}

// Event subscription management
interface EventSubscription {
  id: string
  callback: (eventName: string, payload: any) => void
}

export interface IpLocationResponse {
  latitude: number
  longitude: number
  city: string
  region: string
  country: string
  country_code: string
}

class ApiService {
  private eventSubscriptions: Map<string, EventSubscription> = new Map()
  private tauriListeners: Array<() => void> = []
  private isEventSystemSetup = false

  // Create a unique subscription ID
  private generateSubscriptionId(): string {
    return Math.random().toString(36).substr(2, 9)
  }

  // Central event dispatcher that all Tauri events go through
  private dispatchEvent(eventName: string, payload: any) {
    console.log(`📡 Dispatching event: ${eventName}`, payload)
    this.eventSubscriptions.forEach((subscription) => {
      subscription.callback(eventName, payload)
    })
  }

  // Setup the Tauri event system only once
  private async setupEventSystem() {
    if (this.isEventSystemSetup || !isTauri()) {
      return
    }

    await loadTauriAPIs()

    if (!tauriListen) {
      console.error('Tauri listen API not available')
      return
    }

    console.log('🔧 Setting up centralized Tauri event system...')

    // Set up listeners for all events once
    const events = [
      'eltord-activated',
      'eltord-deactivated',
      'eltord-error',
      'eltord-log',
    ]

    for (const eventName of events) {
      const unlisten = await tauriListen(eventName, (event: any) => {
        this.dispatchEvent(eventName, event.payload)
      })
      this.tauriListeners.push(unlisten)
    }

    this.isEventSystemSetup = true
    console.log('✅ Centralized Tauri event system setup complete')
  }

  // Public method to subscribe to events
  async subscribeToEvents(
    callback: (eventName: string, payload: any) => void,
  ): Promise<() => void> {
    // Set up the event system if not already done
    await this.setupEventSystem()

    const subscriptionId = this.generateSubscriptionId()
    console.log(`📝 Creating event subscription: ${subscriptionId}`)

    this.eventSubscriptions.set(subscriptionId, {
      id: subscriptionId,
      callback,
    })

    // Return unsubscribe function
    return () => {
      console.log(`🗑️ Removing event subscription: ${subscriptionId}`)
      this.eventSubscriptions.delete(subscriptionId)
    }
  }

  // Cleanup all event listeners (useful for app shutdown)
  private cleanupEventSystem() {
    console.log('🧹 Cleaning up Tauri event system')
    this.tauriListeners.forEach((unlisten) => unlisten())
    this.tauriListeners.length = 0
    this.eventSubscriptions.clear()
    this.isEventSystemSetup = false
  }
  // Eltord methods
  async activateEltord(torrcFile?: string, mode?: 'client' | 'relay'): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('activate_eltord', { 
        torrcFileName: torrcFile,
        mode: mode || 'client'
      })
    } else {
      // Build endpoint based on provided parameters
      let endpoint = `${getApiBaseUrl()}/api/eltord/activate`
      
      if (mode && torrcFile) {
        // Both mode and torrc file specified
        endpoint = `${getApiBaseUrl()}/api/eltord/activate/${encodeURIComponent(mode)}/${encodeURIComponent(torrcFile)}`
      } else if (mode) {
        // Only mode specified
        endpoint = `${getApiBaseUrl()}/api/eltord/activate/${encodeURIComponent(mode)}`
      } else if (torrcFile) {
        // Only torrc file specified (use default client mode)
        endpoint = `${getApiBaseUrl()}/api/eltord/activate/client/${encodeURIComponent(torrcFile)}`
      }
      // If neither specified, use default endpoint
      
      const response = await fetch(endpoint, {
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

  // Enhanced Eltord activation with explicit mode support
  async activateEltordWithMode(options: {
    mode: 'client' | 'relay'
    torrcFile?: string
  }): Promise<string> {
    return this.activateEltord(options.torrcFile, options.mode)
  }

  async deactivateEltord(): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('deactivate_eltord')
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/eltord/deactivate`, {
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

  async deactivateEltordWithMode(mode: 'client' | 'relay'): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      console.log(`📡 [API] Calling deactivate_eltord_with_mode for mode: ${mode}`)
      return await tauriInvoke('deactivate_eltord_with_mode', { mode })
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/eltord/deactivate/${mode}`, {
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
      const response = await fetch(`${getApiBaseUrl()}/api/eltord/status`)
      return await response.json()
    }
  }

  // Tor methods
  async connectTor(): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('connect_tor')
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/tor/connect`, {
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
      const response = await fetch(`${getApiBaseUrl()}/api/tor/disconnect`, {
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
      const response = await fetch(`${getApiBaseUrl()}/api/tor/status`)
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

  // Event listening (only works in Tauri) - DEPRECATED: Use subscribeToEvents instead
  async listenToEvents(callback: (eventName: string, payload: any) => void) {
    console.warn(
      '⚠️ listenToEvents is deprecated, using new subscription system',
    )
    return await this.subscribeToEvents(callback)
  }

  // Log streaming for web mode (Server-Sent Events)
  createLogStream(
    onLog: (log: LogEntry) => void,
    onError?: (error: Error) => void,
  ): () => void {
    if (isTauri()) {
      // In Tauri mode, logs come through events
      console.warn(
        'Log streaming not available in Tauri mode - use listenToEvents instead',
      )
      return () => {}
    }

    const eventSource = new EventSource(`${getApiBaseUrl()}/api/eltord/logs`)

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

  // IP Location lookup
  async lookupIpLocation(ip: string): Promise<IpLocationResponse> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('lookup_ip_location_tauri', { ip })
    } else {
      const response = await fetch(
        `${getApiBaseUrl()}/api/ip/${encodeURIComponent(ip)}`,
      )
      if (!response.ok) {
        const error = await response.json()
        throw new Error(error.error || 'Failed to lookup IP location')
      }
      return await response.json()
    }
  }

  // Bulk IP location lookup (primarily for web API)
  async lookupBulkIpLocations(
    ips: string[],
  ): Promise<Array<IpLocationResponse | { error: string }>> {
    if (isTauri()) {
      // For Tauri, we'll call the single lookup for each IP
      const results = await Promise.allSettled(
        ips.map((ip) => this.lookupIpLocation(ip)),
      )
      return results.map((result) => {
        if (result.status === 'fulfilled') {
          return result.value
        } else {
          return { error: result.reason?.message || 'Unknown error' }
        }
      })
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/ip/bulk`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ips }),
      })
      if (!response.ok) {
        throw new Error('Failed to lookup bulk IP locations')
      }
      const data = await response.json()
      return data.results.map((result: any) => {
        if (result.Ok) {
          return result.Ok
        } else {
          return { error: result.Err || 'Unknown error' }
        }
      })
    }
  }
}

export const apiService = new ApiService()
