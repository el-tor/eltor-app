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
    if (!logs || logs.length === 0) return

    // Get the most recent log
    const latestLog = logs[logs.length - 1]
    const message = latestLog?.message || ''

    // Look for "Bootstrapping X%" pattern (case-insensitive)
    const bootstrapMatch = message.match(/Bootstrapping\s+(\d+)%/i)

    if (bootstrapMatch) {
      const newProgress = parseInt(bootstrapMatch[1], 10)
      console.log(`🚀 Bootstrap progress: ${newProgress}%`)

      setProgress(newProgress)

      // Show bootstrapping when we first detect it (and not yet complete)
      if (!isBootstrapping && newProgress < 100) {
        setIsBootstrapping(true)
      }

      // When we reach 100%, trigger callback
      if (newProgress === 100) {
        console.log('✅ Bootstrapping complete!')
        
        // Brief delay to show 100% before transitioning
        setTimeout(() => {
          setIsBootstrapping(false)
          onComplete?.()
        }, 500)
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
