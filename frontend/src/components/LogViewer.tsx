import React, { useState, useEffect, useRef } from 'react'
import { apiService, LogEntry } from '../services/apiService'
import { useDispatch, useSelector } from '../hooks'

interface LogViewerProps {
  className?: string
  height?: string
  mode?: 'client' | 'relay' // Determines which log state to use
  scroll?: boolean
}

const LogViewer: React.FC<LogViewerProps> = ({
  className = '',
  height = '400px',
  mode = 'client',
  scroll = true,
}) => {
  const [autoScroll, setAutoScroll] = useState(scroll)
  const logsEndRef = useRef<HTMLDivElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const dispatch = useDispatch()
  
  // Get logs from Redux based on mode
  const { logsClient, logsRelay } = useSelector((state) => state.global)
  const logs = mode === 'client' ? logsClient : logsRelay
  
  console.log(`📊 LogViewer (${mode}): Displaying ${logs?.length || 0} logs`)

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

  // Load initial logs from status endpoint (only once per mode)
  useEffect(() => {
    const loadInitialLogs = async () => {
      try {
        console.log(`📥 LogViewer (${mode}): Loading initial logs...`)
        const status = await apiService.getEltordStatus()
        console.log(
          `📥 LogViewer (${mode}): Initial logs loaded:`,
          status.recent_logs?.length || 0,
          'logs',
        )
        
        // Only load initial logs if we don't have any logs yet
        if (status.recent_logs && logs?.length === 0) {
          // Filter initial logs by mode
          const filteredLogs = status.recent_logs.filter(log => {
            if (mode === 'client') {
              return log.mode === 'client' || !log.mode
            } else {
              return log.mode === 'relay'
            }
          })
          
          console.log(`📥 LogViewer (${mode}): Filtered initial logs:`, filteredLogs?.length)
          
          // Dispatch to appropriate store based on mode
          if (mode === 'client') {
            dispatch({ type: 'global/setLogsClient', payload: filteredLogs })
          } else {
            dispatch({ type: 'global/setLogsRelay', payload: filteredLogs })
          }
        }
      } catch (error) {
        console.error(`❌ LogViewer (${mode}): Failed to load initial logs:`, error)
      }
    }

    loadInitialLogs()
  }, [mode, dispatch]) // Remove logs dependency to prevent infinite loops

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
          <span className="text-xs text-gray-400">
            {mode.charAt(0).toUpperCase() + mode.slice(1)} Logs ({logs?.length || 0})
          </span>
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
        {logs?.length === 0 ? (
          <div className="text-gray-500 text-center py-8">
            No {mode} logs yet. Start the eltord {mode} process to see logs here.
          </div>
        ) : (
          logs?.map((log, index) => (
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
              {log.mode && (
                <span className="shrink-0 text-blue-400 text-xs">
                  [{log.mode}]
                </span>
              )}
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
