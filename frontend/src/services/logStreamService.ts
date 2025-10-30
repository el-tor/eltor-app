import { apiService, LogEntry } from './apiService'
import { isTauri } from '../utils/platform'
import { setCircuits, setCircuitInUse, Circuit, Relay, addLogClient, addLogRelay } from '../globalStore'
import { Dispatch } from '@reduxjs/toolkit'

class LogStreamService {
  private clientSetup = false
  private relaySetup = false
  private clientCleanup: (() => void) | undefined
  private relayCleanup: (() => void) | undefined
  private dispatch: Dispatch | null = null
  private initializingClient = false
  private initializingRelay = false
  private clientPaused = true  // Start paused by default
  private relayPaused = true   // Start paused by default

  public async initialize(dispatch: Dispatch, mode: 'client' | 'relay' | 'both' = 'client') {
    this.dispatch = dispatch

    // Don't auto-start streams if paused - just store the dispatch
    // Streams will start when user clicks Play
    console.log(`📋 LogStreamService: Initialized with dispatch, waiting for user to start streaming`)
  }

  private async initializeMode(mode: 'client' | 'relay') {
    const isSetup = mode === 'client' ? this.clientSetup : this.relaySetup
    const isInitializing = mode === 'client' ? this.initializingClient : this.initializingRelay
    const isPaused = mode === 'client' ? this.clientPaused : this.relayPaused
    
    // Don't reinitialize if already setup and not resuming from pause
    if (isSetup && !isPaused) {
      console.log(`🔧 LogStreamService: ${mode} already initialized, skipping`)
      return
    }
    
    if (isInitializing) {
      console.log(`🔧 LogStreamService: ${mode} already initializing, skipping`)
      return
    }

    console.log(`🔧 LogStreamService: Initializing log streaming for mode: ${mode}`)
    console.log(`🔧 LogStreamService: isTauri=${isTauri()}`)

    // Mark as initializing immediately to prevent race conditions
    if (mode === 'client') {
      this.initializingClient = true
    } else {
      this.initializingRelay = true
    }

    try {
      // First, load recent logs to get initial state
      console.log(`📥 LogStreamService (${mode}): Fetching recent logs...`)
      const recentLogs = await apiService.getRecentLogs(mode)
      console.log(`📥 LogStreamService (${mode}): Received ${recentLogs.length} recent logs`)
      
      // Process recent logs - convert to LogEntry and store in Redux, extract circuit info
      recentLogs.forEach(logLine => {
        const logEntry = this.parseLogLine(logLine, mode)
        if (logEntry) {
          this.storeLogInRedux(logEntry, mode)
        }
        this.handleCircuitEvents(logLine)
      })
      console.log(`✅ LogStreamService (${mode}): Processed ${recentLogs.length} recent logs`)

      // Then start streaming new logs
      console.log(`📡 LogStreamService (${mode}): Starting log stream...`)
      const cleanup = await apiService.createLogStream(
        mode,
        (logLine: string) => {
          console.log(`📨 LogStreamService (${mode}): Received log:`, logLine)
          this.distributeLog(logLine, mode)
        },
        (error: Error) => {
          console.error(`❌ LogStreamService (${mode}): Log stream error:`, error)
          this.handleError(error)
        },
      )
      console.log(`📡 LogStreamService (${mode}): Log stream cleanup function created`)
      
      // Store cleanup function
      if (mode === 'client') {
        this.clientCleanup = cleanup
        this.clientSetup = true
        this.initializingClient = false
      } else {
        this.relayCleanup = cleanup
        this.relaySetup = true
        this.initializingRelay = false
      }
      
      console.log(`✅ LogStreamService (${mode}): Log stream setup complete`)
    } catch (error) {
      console.error(`❌ LogStreamService (${mode}): Failed to setup log streaming:`, error)
      // Reset flags on error
      if (mode === 'client') {
        this.clientSetup = false
        this.initializingClient = false
      } else {
        this.relaySetup = false
        this.initializingRelay = false
      }
      throw error
    }
  }

  private parseLogLine(logLine: string, mode: 'client' | 'relay'): LogEntry | null {
    // Try to parse log line into LogEntry format
    // Expected format from eltord logs: timestamp + level + message
    // Example: "2025-10-25T10:30:45Z INFO eltor::client Starting client..."
    
    const match = logLine.match(/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(\w+)\s+(.+)$/)
    
    if (match) {
      const [, timestamp, level, message] = match
      return {
        timestamp,
        level: level.toLowerCase(),
        message,
        source: 'stdout',
        mode,
      }
    }
    
    // If no match, create a simple log entry
    return {
      timestamp: new Date().toISOString(),
      level: 'info',
      message: logLine,
      source: 'stdout',
      mode,
    }
  }

  private storeLogInRedux(logEntry: LogEntry, mode: 'client' | 'relay') {
    if (!this.dispatch) return
    
    // Store in appropriate log store based on mode
    if (mode === 'client') {
      this.dispatch(addLogClient(logEntry))
    } else {
      this.dispatch(addLogRelay(logEntry))
    }
  }

  private distributeLog(logLine: string, mode: 'client' | 'relay') {
    if (!this.dispatch) {
      console.warn('⚠️ LogStreamService: No dispatch available, skipping log')
      return
    }

    // Parse and store log in Redux
    const logEntry = this.parseLogLine(logLine, mode)
    if (logEntry) {
      this.storeLogInRedux(logEntry, mode)
    }

    // Handle circuit events from raw log line
    this.handleCircuitEvents(logLine)
  }

  private handleCircuitEvents(logLine: string) {
    if (!this.dispatch) return

    // Parse circuit events from the log line
    // Look for the EVENT:...:ENDEVENT pattern
    const eventData = logLine.match(/EVENT:(.*?):ENDEVENT/)
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
          case 'CIRCUIT_CLOSED':
            console.log('🔄 LogStreamService: Circuit closed:', parsedData.circuit_id)
            // Could handle circuit removal here
            break
          default:
            console.warn(`⚠️ LogStreamService: Unhandled event type: ${parsedData.event}`)
            break
        }
      } catch (error) {
        console.error('❌ LogStreamService: Failed to parse event data:', error, logLine)
      }
    }
    
    // Also look for raw Tor circuit events in the logs
    // Example: "650 CIRC 123 BUILT $fingerprint1~name1,$fingerprint2~name2"
    const torCircuitMatch = logLine.match(/650 CIRC (\d+) BUILT (.+)/)
    if (torCircuitMatch) {
      const circuitId = parseInt(torCircuitMatch[1], 10)
      const path = torCircuitMatch[2]
      console.log(`🔄 LogStreamService: Tor circuit built: ${circuitId} -> ${path}`)
      
      // Parse relay path - create minimal Relay objects
      const relays = path.split(',').map((relay, hop) => {
        const parts = relay.split('~')
        return {
          fingerprint: parts[0].replace('$', ''),
          nickname: parts[1] || 'Unknown',
          hop,
          // Fill in required fields with defaults/placeholders
          bandwidth: 0,
          contact: null,
          ip: '',
          payment_bip353: null,
          payment_bolt11_lightning_address: null,
          payment_bolt11_lnurl: null,
          payment_bolt12_offer: null,
          payment_handshake_fee: null,
          payment_handshake_fee_payhash: '',
          payment_handshake_fee_preimage: '',
          payment_id_hashes_10: [],
          payment_interval_rounds: 0,
          payment_interval_seconds: 0,
          payment_rate_msats: 0,
          port: 0,
          relay_tag: '',
        }
      })
      
      const circuit: Circuit = {
        id: circuitId,
        relays,
      }
      
      this.dispatch(setCircuitInUse(circuit))
      this.dispatch(setCircuits([circuit]))
    }
  }

  private handleError(error: Error) {
    console.error('❌ LogStreamService: Stream error:', error)
    // Could implement reconnection logic here
  }

  public shutdown() {
    console.log('🧹 LogStreamService: Shutting down')
    if (this.clientCleanup) {
      console.log('🧹 LogStreamService: Cleaning up client stream')
      this.clientCleanup()
      this.clientCleanup = undefined
    }
    if (this.relayCleanup) {
      console.log('🧹 LogStreamService: Cleaning up relay stream')
      this.relayCleanup()
      this.relayCleanup = undefined
    }
    this.clientSetup = false
    this.relaySetup = false
    this.initializingClient = false
    this.initializingRelay = false
    this.dispatch = null
  }

  public isInitialized(mode?: 'client' | 'relay'): boolean {
    if (mode === 'client') return this.clientSetup
    if (mode === 'relay') return this.relaySetup
    return this.clientSetup || this.relaySetup
  }

  public async pause(mode: 'client' | 'relay') {
    console.log(`⏸️ LogStreamService: Pausing ${mode} logs`)
    if (mode === 'client') {
      this.clientPaused = true
      if (this.clientCleanup) {
        console.log('🧹 LogStreamService: Closing client stream')
        this.clientCleanup()
        this.clientCleanup = undefined
      }
      this.clientSetup = false
      // For Tauri, also call the stop command
      if (isTauri()) {
        await apiService.stopLogStream('client')
      }
    } else {
      this.relayPaused = true
      if (this.relayCleanup) {
        console.log('🧹 LogStreamService: Closing relay stream')
        this.relayCleanup()
        this.relayCleanup = undefined
      }
      this.relaySetup = false
      // For Tauri, also call the stop command
      if (isTauri()) {
        await apiService.stopLogStream('relay')
      }
    }
  }

  public async resume(mode: 'client' | 'relay') {
    console.log(`▶️ LogStreamService: Resuming ${mode} logs`)
    if (mode === 'client') {
      this.clientPaused = false
    } else {
      this.relayPaused = false
    }
    
    // Reinitialize the stream
    await this.initializeMode(mode)
  }

  public isPaused(mode: 'client' | 'relay'): boolean {
    return mode === 'client' ? this.clientPaused : this.relayPaused
  }
}

// Export singleton instance
export const logStreamService = new LogStreamService()
