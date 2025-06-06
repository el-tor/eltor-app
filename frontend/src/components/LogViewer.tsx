import React, { useState, useEffect, useRef } from 'react'
import { apiService, LogEntry } from '../services/apiService'
import { isTauri } from '../utils/platform'
import { Circuit, setCircuitInUse, setCircuits } from '../globalStore'
import { useDispatch, useSelector } from '../hooks'

interface LogViewerProps {
  className?: string
  height?: string
  logs: LogEntry[]
  setLogs: React.Dispatch<React.SetStateAction<LogEntry[]>>
}

const LogViewer: React.FC<LogViewerProps> = ({
  className = '',
  height = '400px',
  logs,
  setLogs,
}) => {
  const [isConnected, setIsConnected] = useState(false)
  const [autoScroll, setAutoScroll] = useState(true)
  const logsEndRef = useRef<HTMLDivElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const dispatch = useDispatch()

  const eltorLog = `
%@@@@@@@@@@@@@%%%@@@@%%%%%%%%%@@@@@@@@@@@@@@@@%%%%%@@@@@@%%%%%%%@@@@@@@@@@%%%%%%
@#***********#%%%****%%%%%%%@%********###**#%%%%@@%#++*#%@@%%%%%#*******##%@@%%%
@=:::::::::::-%@#::::#@%%%%@@#:::::::::::::-%@%@%+::::::-*%@%%%@+:::::::::-+%@%%
@=:::::::::::-@@#::::#@%%%%@@%:::::::::::::-%@@*::::::::::-#@%%@+:::::::::::-%%%
@=:::::::::::-@@#::::#@%%%%%@%:::::::::::::-@@+:::::::::::::#@%@+::::::::::::+@%
@=::::======-=%@#::::#@%%%%%@#-----::::----=@#::::::::::::::-%@@+::::-=--:::::#%
@=:::=@@@@@@@@@@#::::#@%%%%%%%@@@@#::::#@@@%%-::::-#%%%+:::::+@@+:::-%@@%+::::*@
@=:::=@@@@@@@@@@#::::#@%%%%@@@@%%@#::::#@%%@#::::-%@@@@@#::::-%@+:::-%@@@%::::+@
@=::::------*@@@#::::#@%%@%###%@%@#::::#@%%@+::::#@%%%%@@=::::%@+:::-@@@@*::::*@
@=::::::::::+@@@#::::#@%@%=--:+@%@#::::#@%%@+::::%@%%%%%@*::::#@+::::+++=:::::#@
@+::::::::::+@@@#::::#@%%%****#%%@#::::#@%%@+::::%@%%%%%@*::::#@+::::::::::::=%%
@+::::::::::+@@@#::::#@%%%@@@@@%%@#::::#@%%@+::::#@%%%%%@=::::%@+:::::::::::-%@%
@+:::=%%%%%%%@@@#::::#@%%%%@@%%%%@*::::#@%%@#::::-%@%%%@*::::-%@+::::::::---%@%%
@+:--=@%%%%@%%%@#:--:*%%%%%%%%%%%@*:--:#@%%%%-:--:-#%%%+:---:*@@+:---+*----:#@%%
@+:-----------+@#:-----------%@%%@*:--:#@%%%@#:--------:---:-%%@+:---%@+:----%%%
@+-----------:+@#:----------:#@%%@*:--:#@%%%%@*:----------:-%@%%+:---%@%----:+@%
@+------------+@#-----------:#%%%@*:--:#@%%%%%@*-:-------:=%@%%%+:---%%@#:----#@
@=:::--------:+@#::---:::--::#@%%@*::::#@%%%%%%@%+-------*%@%%%%+:::-%%%@*:--:-%
@############*#@%**#*********%%%%%#****%%%%%%%%%%@%#***#%@%%%%%%#****%%%%%**##*#
%@@@@@@@@@@@@@@%%@@%@@@@@%%@@%%%%%%@@@@%%%%%%%%%%%%@@@@%%%%%%%%%%@@@@%%%%%@@@%%%
`

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' })
    }
  }, [logs, autoScroll])

  // Check if user has scrolled up to disable auto-scroll
  const handleScroll = () => {
    if (containerRef.current) {
      const { scrollTop, scrollHeight, clientHeight } = containerRef.current
      const isAtBottom = scrollTop + clientHeight >= scrollHeight - 10
      setAutoScroll(isAtBottom)
    }
  }

  useEffect(() => {
    let cleanup: (() => void) | undefined
    let isEffectActive = true // Flag to prevent stale updates
    let lastCircuitId = 0 // Reset lastCircuitId on mount

    const setupLogStreaming = async () => {
      console.log('🔧 LogViewer: Setting up log streaming, isTauri:', isTauri())

      if (isTauri()) {
        // Tauri mode - use new subscription system to prevent duplicates
        cleanup = await apiService.subscribeToEvents((eventName, payload) => {
          // Only process if this effect is still active
          if (!isEffectActive) return

          console.log('📡 LogViewer: Received Tauri event:', eventName, payload)
          if (eventName === 'eltord-log') {
            console.log('📝 LogViewer: Adding new log:', payload)
            // Use functional update to avoid stale closure issues
            setLogs((currentLogs: LogEntry[]) => {
              const incomingLog = payload as LogEntry
              // Prevent duplicates by checking for unique timestamp (or use another unique property)
              if (
                currentLogs.some(
                  (log) =>
                    log.timestamp === incomingLog.timestamp &&
                    log.message === incomingLog.message,
                )
              ) {
                return currentLogs
              }
              const newLogs = [...currentLogs, incomingLog]
              console.log('📊 Updated logs count:', newLogs.length)
              lastCircuitId = handleEvent(
                incomingLog.message,
                lastCircuitId,
                dispatch,
              )
              return newLogs
            })
          }
        })
        setIsConnected(true)
        console.log('✅ LogViewer: Tauri event subscription setup complete')
      } else {
        // Web mode - use Server-Sent Events
        cleanup = apiService.createLogStream(
          (log: LogEntry) => {
            if (!isEffectActive) return // Prevent stale updates
            console.log('📡 LogViewer: Received SSE log:', log)
            console.log('📝 LogViewer: Adding new log:', log)
            lastCircuitId = handleEvent(log.message, lastCircuitId, dispatch)
            setLogs((currentLogs: LogEntry[]) => {
              const newLogs = [...currentLogs, log]
              console.log('📊 Updated logs count:', newLogs.length)
              return newLogs
            })
          },
          (error: Error) => {
            console.error('❌ Log stream error:', error)
            setIsConnected(false)
          },
        )
        setIsConnected(true)
        console.log('✅ LogViewer: SSE log stream setup complete')
      }
    }

    setupLogStreaming().catch((error) => {
      console.error('❌ Failed to setup log streaming:', error)
      setIsConnected(false)
    })

    return () => {
      console.log('🧹 LogViewer: Cleaning up log streaming listeners')
      isEffectActive = false // Mark effect as inactive
      cleanup?.()
      setIsConnected(false)
    }
  }, [])

  // Load initial logs from status endpoint
  useEffect(() => {
    const loadInitialLogs = async () => {
      try {
        console.log('Loading initial logs...')
        const status = await apiService.getEltordStatus()
        console.log(
          'Initial logs loaded:',
          status.recent_logs?.length || 0,
          'logs',
        )
        if (status.recent_logs) {
          setLogs(status.recent_logs)
        }
      } catch (error) {
        console.error('Failed to load initial logs:', error)
      }
    }

    loadInitialLogs()
  }, [setLogs])

  const clearLogs = () => {
    setLogs([])
  }

  const getLevelColor = (level: string) => {
    switch (level.toUpperCase()) {
      case 'ERROR':
        return 'red'
      case 'WARN':
      case 'WARNING':
        return 'yellow'
      case 'INFO':
        return 'white'
      case 'DEBUG':
        return 'white'
      default:
        return 'white'
    }
  }

  const getSourceColor = (source: string) => {
    switch (source) {
      case 'stdout':
        return 'text-green-400'
      case 'stderr':
        return 'text-red-400'
      case 'system':
        return 'text-purple-400'
      default:
        return 'text-gray-400'
    }
  }

  const formatTimestamp = (timestamp: string) => {
    try {
      const date = new Date(timestamp)
      return date.toLocaleTimeString()
    } catch {
      return timestamp
    }
  }

  return (
    <div
      className={`bg-gray-900 border border-gray-700 rounded-lg ${className}`}
    >
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-gray-700">
        <div className="flex items-center gap-2">
    
          <label className="flex items-center gap-1 text-xs text-gray-400" style={{float: 'right'}}>
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="w-3 h-3"
            />
            Auto-scroll
          </label>
          <h3 className="font-semibold text-white">Eltord Logs</h3>
          <div
            className={`w-2 h-2 rounded-full ${
              isConnected ? 'bg-green-500' : 'bg-red-500'
            }`}
          />          
        </div>
      </div>

      {/* Logs container */}
      <div
        ref={containerRef}
        className="overflow-y-auto p-2 font-mono text-xs"
        // style={{ height }}
        onScroll={handleScroll}
      >
        {logs.length === 0 ? (
          <div className="text-gray-500 text-center py-8">
            No logs yet. Start the eltord process to see logs here.
          </div>
        ) : (
          logs.map((log, index) => (
            <div key={index} className="flex gap-2 py-1 hover:bg-gray-800">
              <span className="text-gray-500 shrink-0">
                {formatTimestamp(log.timestamp)}
              </span>
              <span
                className={`shrink-0 font-bold ${getLevelColor(log.level)}`}
              >
                {log.level.toUpperCase()}
              </span>
              <span className={`shrink-0 ${getSourceColor(log.source)}`}>
                [{log.source}]
              </span>
              <span className="text-gray-200 break-all" style={{color: getLevelColor(log.level)}}> {log.message}</span>
            </div>
          ))
        )}
        <span className="blink-cursor">&nbsp;</span>
        <div ref={logsEndRef} />
      </div>
    </div>
  )
}

export default LogViewer

export function handleEvent(
  event: string,
  lastCircuitId: number,
  dispatch: any,
) {
  // parse the event data EVENT:{event_type, data}:ENDEVENT
  const eventData = event.match(/EVENT:(.*):ENDEVENT/)
  if (eventData && eventData[1]) {
    try {
      const parsedData = JSON.parse(eventData[1])
      switch (parsedData.event) {
        case 'CIRCUIT_BUILT':
          const circuit: Circuit = {
            id: parsedData.circuit_id,
            relays: parsedData.relays,
          }
          console.log(JSON.stringify(circuit, null, 2))
          //if (lastCircuitId !== circuit.id) {
          console.info('Handle Event', circuit)
          dispatch(setCircuitInUse(circuit))
          dispatch(setCircuits([circuit]))
          // if (circuit.id !== circuit.circuitInUse.id) {
          //   dispatch(setCircuitInUse(circuitResp.circuitInUse))
          // }
          //}
          //lastCircuitId = circuit.id
          break
        default:
          console.warn(`Unhandled event type: ${parsedData.event}`)
          break
      }
    } catch (error) {
      console.error('Failed to parse event data:', error)
    }
  }
  return lastCircuitId
}
