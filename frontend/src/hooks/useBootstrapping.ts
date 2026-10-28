import { useState, useEffect } from 'react'
import type { LogEntry } from '../services/apiService'

interface UseBootstrappingOptions {
  logs: LogEntry[]
  onComplete?: () => void
}

interface UseBootstrappingReturn {
  progress: number
  isBootstrapping: boolean
  reset: () => void
  start: () => void
}

/**
 * Hook to monitor Tor bootstrapping progress from log messages
 * 
 * Watches for "Bootstrapping X%" patterns in logs and tracks progress.
 * When bootstrapping reaches 100%, it triggers an optional callback.
 * 
 * @param logs - Array of log entries to monitor
 * @param onComplete - Optional callback when bootstrapping reaches 100%
 * @returns Object containing progress (0-100), isBootstrapping flag, and reset function
 */
export const useBootstrapping = ({
  logs,
  onComplete,
}: UseBootstrappingOptions): UseBootstrappingReturn => {
  const [progress, setProgress] = useState(0)
  const [isBootstrapping, setIsBootstrapping] = useState(false)

  useEffect(() => {
    if (!isBootstrapping || !logs || logs.length === 0) return

    // React can batch several log events. Inspect the latest progress entry,
    // not just the last line, which may already be a wallet/status message.
    const bootstrapMatch = [...logs].reverse().map(log =>
      log.message.match(/Bootstrapping\s+(\d+)%/i) ||
      log.message.match(/Tor ready:\s+(100)% bootstrap/i)
    ).find(Boolean)

    if (bootstrapMatch) {
      const newProgress = parseInt(bootstrapMatch[1], 10)
      console.log(`🚀 Bootstrap progress: ${newProgress}%`)

      setProgress(newProgress)

      // When we reach 100%, trigger callback
      if (newProgress === 100) {
        console.log('✅ Bootstrapping complete!')
        
        setIsBootstrapping(false)
        onComplete?.()
      }
    }
  }, [logs, isBootstrapping, onComplete])

  const reset = () => {
    setProgress(0)
    setIsBootstrapping(false)
  }

  const start = () => {
    setProgress(1)
    setIsBootstrapping(true)
  }

  return {
    progress,
    isBootstrapping,
    reset,
    start,
  }
}
