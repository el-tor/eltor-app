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

// Event subscription management
interface EventSubscription {
  id: string
  callback: (eventName: string, payload: any) => void
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
    const events = ['eltord-activated', 'eltord-deactivated', 'eltord-error', 'eltord-log']
    
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
  async subscribeToEvents(callback: (eventName: string, payload: any) => void): Promise<() => void> {
    // Set up the event system if not already done
    await this.setupEventSystem()
    
    const subscriptionId = this.generateSubscriptionId()
    console.log(`📝 Creating event subscription: ${subscriptionId}`)
    
    this.eventSubscriptions.set(subscriptionId, {
      id: subscriptionId,
      callback
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
    this.tauriListeners.forEach(unlisten => unlisten())
    this.tauriListeners.length = 0
    this.eventSubscriptions.clear()
    this.isEventSystemSetup = false
  }
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

  // Event listening (only works in Tauri) - DEPRECATED: Use subscribeToEvents instead
  async listenToEvents(callback: (eventName: string, payload: any) => void) {
    console.warn('⚠️ listenToEvents is deprecated, using new subscription system')
    return await this.subscribeToEvents(callback)
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
