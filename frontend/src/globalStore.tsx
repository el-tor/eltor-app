import { createSlice } from '@reduxjs/toolkit'
import type { PayloadAction } from '@reduxjs/toolkit'
import { LogEntry } from './services/apiService'

export {
  type GlobalState,
  type Circuit,
  type Relay,
  globalStore,
  globalReducer,
  setLogsClient,
  setLogsRelay,
  addLogClient,
  addLogRelay,
  clearLogsClient,
  clearLogsRelay,
  clearAllLogs,
  setTorActive,
  setRelayActive,
  setClientActive,
  setActiveMode,
  setCircuits,
  setRelays,
  setCircuitInUse,
  setMyIp,
  setRelayEnabled,
  setClientEnabled,
}

 const MAX_LOGS = 2000

// 1. State
interface GlobalState {
  logsClient: LogEntry[]
  logsRelay: LogEntry[]
  clientActive: boolean
  relayActive: boolean
  torActive: boolean // Deprecated: for backward compatibility
  // New: Track which mode is currently running (if any)
  activeMode: 'client' | 'relay' | null
  circuits: Array<Circuit> | null
  circuitInUse: Circuit
  relays: Array<Relay>
  myIp: string
  relayEnabled: boolean
  clientEnabled: boolean
}

const initialState: GlobalState = {
  logsClient: [
    {
      level: "",
      source: "",
      timestamp: new Date().toISOString(),
      message: ``,
    },
  ],
  logsRelay: [],
  clientActive: false,
  relayActive: false,
  torActive: false, // Deprecated: for backward compatibility
  activeMode: null, // Track which mode is currently running
  circuits: [],
  circuitInUse: {} as Circuit,
  relays: [],
  myIp: '166.205.90.66',
  relayEnabled: false,
  clientEnabled: true,
}

// 2. Slice and Reducers
const globalStore = createSlice({
  name: 'global',
  initialState,
  reducers: {
    setLogsClient: (state, action: PayloadAction<LogEntry[]>) => {
      state.logsClient = action.payload
    },
    setLogsRelay: (state, action: PayloadAction<LogEntry[]>) => {
      state.logsRelay = action.payload
    },
    addLogClient: (state, action: PayloadAction<LogEntry>) => {
      // Prevent duplicates by checking for unique timestamp + message
      const newLog = action.payload
      const isDuplicate = state.logsClient.some(
        (log) =>
          log.timestamp === newLog.timestamp &&
          log.message === newLog.message
      )
      if (!isDuplicate) {
        state.logsClient.push(newLog)

        // Keep only the last 2000 logs to prevent memory bloat and UI freeze
        // This is critical for long-running sessions with heartbeat logs
        if (state.logsClient.length > MAX_LOGS) {
          state.logsClient = state.logsClient.slice(-MAX_LOGS)
        }
      }
    },
    addLogRelay: (state, action: PayloadAction<LogEntry>) => {
      // Prevent duplicates by checking for unique timestamp + message
      const newLog = action.payload
      const isDuplicate = state.logsRelay.some(
        (log) =>
          log.timestamp === newLog.timestamp &&
          log.message === newLog.message
      )
      if (!isDuplicate) {
        state.logsRelay.push(newLog)
        if (state.logsRelay.length > MAX_LOGS) {
          state.logsRelay = state.logsRelay.slice(-MAX_LOGS)
        }
      }
    },
    clearLogsClient: (state) => {
      state.logsClient = []
    },
    clearLogsRelay: (state) => {
      state.logsRelay = []
    },
    clearAllLogs: (state) => {
      state.logsClient = []
      state.logsRelay = []
    },
    setTorActive: (state, action: PayloadAction<boolean>) => {
      state.torActive = action.payload
    },
    setClientActive: (state, action: PayloadAction<boolean>) => {
      state.clientActive = action.payload
    },
    setRelayActive: (state, action: PayloadAction<boolean>) => {
      state.relayActive = action.payload
    },
    setActiveMode: (state, action: PayloadAction<'client' | 'relay' | null>) => {
      state.activeMode = action.payload
    },
    setCircuits: (state, action: PayloadAction<Array<Circuit> | null>) => {
      state.circuits = action.payload
    },
    setCircuitInUse: (state, action: PayloadAction<Circuit>) => {
      state.circuitInUse = action.payload
    },
    setRelays: (state, action: PayloadAction<Array<Relay>>) => {
      state.relays = action.payload
    },
    setMyIp: (state, action: PayloadAction<string>) => {
      state.myIp = action.payload
    },
    setRelayEnabled: (state, action: PayloadAction<boolean>) => {
      state.relayEnabled = action.payload
    },
    setClientEnabled: (state, action: PayloadAction<boolean>) => {
      state.clientEnabled = action.payload
    },
  },
  extraReducers: (builder) => {},
})

const globalReducer = globalStore.reducer
// Action creators are generated for each case reducer function
const {
  setLogsClient,
  setLogsRelay,
  addLogClient,
  addLogRelay,
  clearLogsClient,
  clearLogsRelay,
  clearAllLogs,
  setTorActive,
  setClientActive,
  setRelayActive,
  setActiveMode,
  setCircuits,
  setRelays,
  setCircuitInUse,
  setMyIp,
  setRelayEnabled,
  setClientEnabled,
} = globalStore.actions

type Circuit = {
  id: number
  relays: Relay[] // usually 3 Guard, Middle, Exit
}

type Relay = {
  bandwidth: number
  contact: string | null
  fingerprint: string
  hop: number
  ip: string
  nickname: string
  payment_bip353: string | null
  payment_bolt11_lightning_address: string | null
  payment_bolt11_lnurl: string | null
  payment_bolt12_offer: string | null
  payment_handshake_fee: number | null
  payment_handshake_fee_payhash: string
  payment_handshake_fee_preimage: string
  payment_id_hashes_10: string[]
  payment_interval_rounds: number
  payment_interval_seconds: number
  payment_rate_msats: number
  port: number
  relay_tag: string
}
