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
  height = '300px',
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
        return 'text-red-400'
      case 'WARN':
      case 'WARNING':
        return 'text-yellow-400'
      case 'INFO':
        return 'text-gray-200'
      case 'DEBUG':
        return 'text-gray-400'
      default:
        return 'text-gray-200'
    }
  }

  // Clean ANSI color codes and extract code path to show at the end
  const cleanLogMessage = (message: string, level: string, timestamp: string) => {
    let cleaned = message
      // Remove ANSI color codes like [0m, [32m, [38;5;8m, etc.
      .replace(/\x1b\[[0-9;]*m/g, '')
      // Remove duplicate timestamp patterns
      .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z\s+/, '')
      // Clean up extra whitespace
      .replace(/\s+/g, ' ')
      .trim()
    
    // Format timestamp
    const time = new Date(timestamp).toLocaleTimeString()
    
    // Extract code path - look for patterns like "eltor::client::payments_loop" that might be at the start
    const codePathMatch = cleaned.match(/^(?:INFO|ERROR|WARN|DEBUG)\s+([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)\s+(.*)/) ||
                          cleaned.match(/^([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)\s+(.*)/)
    
    if (codePathMatch) {
      const [, codePath, messageBody] = codePathMatch
      return `${time} - ${messageBody} PM-[${level.toUpperCase()} ${codePath}]`
    }
    
    // Remove log level indicators if no code path found
    cleaned = cleaned.replace(/^(INFO|ERROR|WARN|DEBUG)\s+/, '')
    
    // If no code path found, just add timestamp and level
    return `${time} - ${cleaned} PM-[${level.toUpperCase()}]`
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
      className={`bg-gray-900 border border-gray-700 rounded-lg min-w-[800px] w-full max-w-6xl ${className}`}
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
          logs
            ?.filter((log) => !log.message.includes('`window'))
            ?.map((log, index) => {
            const cleanedMessage = cleanLogMessage(log.message, log.level, log.timestamp)
            const isError = log.level.toUpperCase() === 'ERROR'
            
            return (
              <div key={index} className={`flex gap-2 py-1 hover:bg-gray-800 ${isError ? 'bg-red-900/20' : ''}`}>
                <span className={`break-all ${getLevelColor(log.level)}`}>
                  {cleanedMessage}
                </span>
              </div>
            )
          })
        )}
        <span className="blink-cursor">&nbsp;</span>
        <div ref={logsEndRef} />
      </div>
    </div>
  )
}

export default LogViewer
