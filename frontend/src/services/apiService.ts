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
  // In Tauri mode, we need to use the backend server port, not the frontend port
  if (isTauri()) {
    // Tauri dev mode: frontend is on 1420, backend is on 5174
    // Tauri prod mode: both should be on the same port (backend serves frontend)
    const backendUrl = `${window.location.protocol}//localhost:5174`
    console.log('apiService - Using Tauri backend URL:', backendUrl)
    return backendUrl
  }
  
  // In web mode, frontend and backend are on the same server
  const currentUrl = `${window.location.protocol}//${window.location.host}`
  console.log('apiService - Using current location as API base:', currentUrl)
  return currentUrl
}

export interface EltordStatus {
  running: boolean
  client_running: boolean
  relay_running: boolean
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
    console.log(`📡 Number of subscribers: ${this.eventSubscriptions.size}`)
    
    let callbackCount = 0
    this.eventSubscriptions.forEach((subscription) => {
      console.log(`📞 Calling subscription ${subscription.id} with event ${eventName}`)
      subscription.callback(eventName, payload)
      callbackCount++
    })
    
    console.log(`📡 Dispatched to ${callbackCount} callbacks`)
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
    console.log(`📝 Total subscriptions before add: ${this.eventSubscriptions.size}`)

    this.eventSubscriptions.set(subscriptionId, {
      id: subscriptionId,
      callback,
    })
    
    console.log(`📝 Total subscriptions after add: ${this.eventSubscriptions.size}`)
    console.log(`📝 Event system setup: ${this.isEventSystemSetup}`)

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
  async activateEltord(
    mode: 'client' | 'relay' | 'both',
  ): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('activate_eltord_invoke', {
        mode: mode || 'client',
      })
    } else {
      // Build endpoint based on provided parameters
      let endpoint = `${getApiBaseUrl()}/api/eltord/activate`
      
        // Only mode specified
        endpoint = `${getApiBaseUrl()}/api/eltord/activate/${encodeURIComponent(
          mode,
        )}`
      

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

  async deactivateEltord(mode: 'client' | 'relay' | 'both'): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      console.log(
        `📡 [API] Calling deactivate_eltord for mode: ${mode}`,
      )
      return await tauriInvoke('deactivate_eltord_invoke', { mode })
    } else {
      const response = await fetch(
        `${getApiBaseUrl()}/api/eltord/deactivate/${mode}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
        },
      )

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
      return await tauriInvoke('get_eltord_status_invoke')
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/eltord/status`)
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

  // Get recent logs (last 100 lines)
  async getRecentLogs(mode: 'client' | 'relay' = 'client'): Promise<string[]> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('get_eltord_logs_invoke', { mode })
    } else {
      const response = await fetch(
        `${getApiBaseUrl()}/api/eltord/logs/${mode}`,
      )
      if (!response.ok) {
        throw new Error('Failed to fetch recent logs')
      }
      const data = await response.json()
      return data.logs
    }
  }

  // Log streaming for both web (SSE) and Tauri (events)
  async createLogStream(
    mode: 'client' | 'relay',
    onLog: (log: string) => void,
    onError?: (error: Error) => void,
  ): Promise<() => void> {
    if (isTauri()) {
      // In Tauri mode, use the stream command which emits 'eltord-log' events
      await loadTauriAPIs()
      
      console.log(`📡 Starting Tauri log stream for mode: ${mode}`)
      console.log(`📡 Tauri APIs loaded:`, { 
        tauriInvoke: !!tauriInvoke, 
        tauriListen: !!tauriListen 
      })
      
      // Subscribe to log events through the centralized event system FIRST
      console.log(`📡 Setting up event subscription for mode: ${mode}`)
      const unsubscribe = await this.subscribeToEvents((eventName, payload) => {
        console.log(`📬 Event received: ${eventName}`, payload)
        if (eventName === 'eltord-log') {
          console.log(`📨 Tauri received log event for ${mode}:`, payload)
          // Payload is the log line string
          onLog(payload)
        } else {
          console.log(`⚠️ Ignoring event: ${eventName}`)
        }
      })
      console.log(`✅ Event subscription created`)
      
      // Then start the stream (this will emit 'eltord-log' events)
      console.log(`📡 Invoking stream_eltord_logs_invoke for mode: ${mode}`)
      tauriInvoke('stream_eltord_logs_invoke', { mode })
        .then(() => {
          console.log(`✅ stream_eltord_logs_invoke completed successfully for mode: ${mode}`)
        })
        .catch((error: any) => {
          console.error(`❌ Failed to start Tauri log stream for ${mode}:`, error)
          onError?.(error)
        })

      console.log(`✅ Tauri log stream setup complete for mode: ${mode}`)
      return unsubscribe
    } else {
      // In Web mode, use Server-Sent Events
      const url = `${getApiBaseUrl()}/api/eltord/logs/stream/${mode}`
      console.log(`📡 Creating SSE connection to: ${url}`)
      
      const eventSource = new EventSource(url)

      console.log(`📡 SSE EventSource created, readyState: ${eventSource.readyState}`)

      // Listen for connection open
      eventSource.addEventListener('open', () => {
        console.log(`✅ SSE connection opened for mode: ${mode}`)
      })

      // Listen for "log" events (backend sends with .event("log"))
      eventSource.addEventListener('log', (event: MessageEvent) => {
        console.log(`📨 SSE received log event:`, event.data)
        try {
          // The backend sends the log line as plain text
          onLog(event.data)
        } catch (error) {
          console.error('Failed to process log entry:', error)
          onError?.(new Error('Failed to process log entry'))
        }
      })

      // Listen for any message (including keep-alive)
      eventSource.addEventListener('message', (event: MessageEvent) => {
        console.log(`📨 SSE received message event:`, event.data)
      })

      eventSource.onerror = (error) => {
        console.error(`❌ SSE error for mode ${mode}, readyState: ${eventSource.readyState}`, error)
        console.error('SSE error details:', {
          type: error.type,
          target: error.target,
          readyState: eventSource.readyState,
        })
        onError?.(new Error('Log stream connection error'))
        // Don't close immediately, let it try to reconnect
      }

      // Return cleanup function
      return () => {
        console.log(`🧹 Closing SSE log stream for mode: ${mode}`)
        eventSource.close()
      }
    }
  }

  // Stop log streaming (Tauri only - for web mode, just close the EventSource via cleanup function)
  async stopLogStream(mode: 'client' | 'relay'): Promise<void> {
    if (isTauri()) {
      await loadTauriAPIs()
      console.log(`🛑 Stopping Tauri log stream for mode: ${mode}`)
      await tauriInvoke('stop_eltord_logs_invoke', { mode })
    }
    // For web mode, the cleanup function returned by createLogStream handles closing
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

  // Debug info
  async getDebugInfo(): Promise<any> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('get_debug_info')
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/debug`)
      if (!response.ok) {
        const error = await response.json()
        throw new Error(error.error || 'Failed to get debug info')
      }
      return await response.json()
    }
  }
}

export const apiService = new ApiService()
