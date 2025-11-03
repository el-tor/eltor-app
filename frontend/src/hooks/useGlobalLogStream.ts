import { useEffect } from 'react'
import { useDispatch, useSelector } from '../hooks'
import { logStreamService } from '../services/logStreamService'

// Global flag to prevent double initialization (survives React StrictMode)
let globalInitialized = false

/**
 * Hook to initialize the global log streaming service.
 * Should be called once at the app level.
 * Automatically determines which modes to stream based on enabled settings.
 */
export function useGlobalLogStream() {
  const dispatch = useDispatch()
  const { clientEnabled, relayEnabled } = useSelector((state) => state.global)
  
  // Determine which modes to stream logs for
  const mode: 'client' | 'relay' | 'both' = 
    clientEnabled && relayEnabled ? 'both' :
    relayEnabled ? 'relay' : 'client'

  useEffect(() => {
    // Prevent double initialization in React StrictMode
    if (globalInitialized) {
      console.log('🔧 useGlobalLogStream: Already initialized globally, skipping')
      return
    }
    
    console.log(`🚀 useGlobalLogStream: Initializing global log service for mode: ${mode}`)
    
    const initializeService = async () => {
      try {
        globalInitialized = true
        await logStreamService.initialize(dispatch, mode)
        console.log(`✅ useGlobalLogStream: Global log service initialized for ${mode}`)
      } catch (error) {
        console.error('❌ useGlobalLogStream: Failed to initialize:', error)
        globalInitialized = false
      }
    }

    initializeService()

    // Cleanup on unmount (app shutdown) - but don't reset global flag in dev mode
    return () => {
      console.log('🧹 useGlobalLogStream: Cleanup called (may be StrictMode)')
      // Don't shutdown in dev mode to prevent double initialization issues
      // In production, this will properly clean up on unmount
      if (import.meta.env.PROD) {
        logStreamService.shutdown()
        globalInitialized = false
      }
    }
  }, [dispatch, mode])
}
