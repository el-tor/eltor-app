import React, { useState, useEffect, useRef } from 'react'
import { apiService, LogEntry } from '../services/apiService'
import { isTauri } from '../utils/platform'

interface LogViewerProps {
  className?: string
  height?: string
  logs: LogEntry[]
  setLogs: (logs: LogEntry[]) => void
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

    const setupLogStreaming = async () => {
      console.log('Setting up log streaming, isTauri:', isTauri())
      
      if (isTauri()) {
        // Tauri mode - listen to events
        cleanup = await apiService.listenToEvents((eventName, payload) => {
          console.log('Received Tauri event:', eventName, payload)
          if (eventName === 'eltord-log') {
            console.log('LogViewer: Adding new log:', payload)
            // Use functional update to avoid stale closure issues
            setLogs(currentLogs => {
              const newLogs = [...currentLogs, payload as LogEntry]
              console.log('Updated logs count:', newLogs.length)
              return newLogs
            })
          }
        })
        setIsConnected(true)
        console.log('Tauri event listener setup complete')
      } else {
        // Web mode - use Server-Sent Events
        cleanup = apiService.createLogStream(
          (log: LogEntry) => {
            console.log('Received SSE log:', log)
            console.log('LogViewer: Adding new log:', log)
            setLogs(currentLogs => {
              const newLogs = [...currentLogs, log]
              console.log('Updated logs count:', newLogs.length)
              return newLogs
            })
          },
          (error: Error) => {
            console.error('Log stream error:', error)
            setIsConnected(false)
          },
        )
        setIsConnected(true)
        console.log('SSE log stream setup complete')
      }
    }

    setupLogStreaming().catch(error => {
      console.error('Failed to setup log streaming:', error)
      setIsConnected(false)
    })

    return () => {
      console.log('Cleaning up log streaming listeners')
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
        console.log('Initial logs loaded:', status.recent_logs?.length || 0, 'logs')
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
        return 'text-red-400'
      case 'WARN':
      case 'WARNING':
        return 'text-yellow-400'
      case 'INFO':
        return 'text-blue-400'
      case 'DEBUG':
        return 'text-gray-400'
      default:
        return 'text-gray-300'
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
          <h3 className="font-semibold text-white">Eltord Logs</h3>
          <div
            className={`w-2 h-2 rounded-full ${
              isConnected ? 'bg-green-500' : 'bg-red-500'
            }`}
          />
          <span className="text-xs text-gray-400">
            {isConnected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1 text-xs text-gray-400">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="w-3 h-3"
            />
            Auto-scroll
          </label>
        </div>
      </div>

      {/* Logs container */}
      <div
        ref={containerRef}
        className="overflow-y-auto p-2 font-mono text-xs"
        style={{ height }}
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
              <span className="text-gray-200 break-all">{log.message}</span>
            </div>
          ))
        )}
        <div ref={logsEndRef} />
      </div>
    </div>
  )
}

export default LogViewer
