import { apiService, LogEntry } from './apiService'
import { isTauri } from '../utils/platform'
import { addLogClient, addLogRelay, setCircuits, setCircuitInUse, Circuit } from '../globalStore'
import { Dispatch } from '@reduxjs/toolkit'

class LogStreamService {
  private isSetup = false
  private cleanup: (() => void) | undefined
  private dispatch: Dispatch | null = null

  public async initialize(dispatch: Dispatch) {
    if (this.isSetup) {
      console.log('🔧 LogStreamService: Already initialized, skipping')
      return
    }

    console.log('🔧 LogStreamService: Initializing global log streaming')
    this.dispatch = dispatch
    this.isSetup = true

    try {
      if (isTauri()) {
        // Tauri mode - use subscription system
        this.cleanup = await apiService.subscribeToEvents((eventName, payload) => {
          if (eventName === 'eltord-log') {
            const logEntry = payload as LogEntry
            this.distributeLog(logEntry)
          }
        })
        console.log('✅ LogStreamService: Tauri event subscription setup complete')
      } else {
        // Web mode - use Server-Sent Events
        this.cleanup = apiService.createLogStream(
          (log: LogEntry) => {
            this.distributeLog(log)
          },
          (error: Error) => {
            console.error('❌ LogStreamService: Log stream error:', error)
            this.handleError(error)
          },
        )
        console.log('✅ LogStreamService: SSE log stream setup complete')
      }
    } catch (error) {
      console.error('❌ LogStreamService: Failed to setup log streaming:', error)
      this.isSetup = false
      throw error
    }
  }

  private distributeLog(logEntry: LogEntry) {
    if (!this.dispatch) {
      console.warn('⚠️ LogStreamService: No dispatch available, skipping log')
      return
    }

    // Handle circuit events first
    if (logEntry.message) {
      this.handleCircuitEvents(logEntry.message)
    }

    // Distribute to appropriate log store based on mode
    if (logEntry.mode === 'client' || (!logEntry.mode && logEntry.source !== 'relay')) {
      // Client logs or system logs (default to client)
      this.dispatch(addLogClient(logEntry))
    } else if (logEntry.mode === 'relay') {
      // Relay logs
      this.dispatch(addLogRelay(logEntry))
    } else {
      // Fallback to client for unknown modes
      this.dispatch(addLogClient(logEntry))
    }
  }

  private handleCircuitEvents(message: string) {
    if (!this.dispatch) return

    // Parse circuit events
    const eventData = message.match(/EVENT:(.*):ENDEVENT/)
    if (eventData && eventData[1]) {
      try {
        const parsedData = JSON.parse(eventData[1])
        switch (parsedData.event) {
          case 'CIRCUIT_BUILT':
            const circuit: Circuit = {
              id: parsedData.circuit_id,
              relays: parsedData.relays,
            }
            console.log('🔄 LogStreamService: Circuit built:', circuit)
            this.dispatch(setCircuitInUse(circuit))
            this.dispatch(setCircuits([circuit]))
            break
          default:
            console.warn(`⚠️ LogStreamService: Unhandled event type: ${parsedData.event}`)
            break
        }
      } catch (error) {
        console.error('❌ LogStreamService: Failed to parse event data:', error)
      }
    }
  }

  private handleError(error: Error) {
    console.error('❌ LogStreamService: Stream error:', error)
    // Could implement reconnection logic here
  }

  public shutdown() {
    console.log('🧹 LogStreamService: Shutting down')
    if (this.cleanup) {
      this.cleanup()
      this.cleanup = undefined
    }
    this.isSetup = false
    this.dispatch = null
  }

  public isInitialized(): boolean {
    return this.isSetup
  }
}

// Export singleton instance
export const logStreamService = new LogStreamService()
