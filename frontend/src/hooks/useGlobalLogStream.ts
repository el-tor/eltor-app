import { useEffect } from 'react'
import { useDispatch } from '../hooks'
import { logStreamService } from '../services/logStreamService'

/**
 * Hook to initialize the global log streaming service.
 * Should be called once at the app level.
 */
export function useGlobalLogStream() {
  const dispatch = useDispatch()

  useEffect(() => {
    console.log('🚀 useGlobalLogStream: Initializing global log service')
    
    const initializeService = async () => {
      try {
        if (!logStreamService.isInitialized()) {
          await logStreamService.initialize(dispatch)
          console.log('✅ useGlobalLogStream: Global log service initialized')
        } else {
          console.log('ℹ️ useGlobalLogStream: Global log service already initialized')
        }
      } catch (error) {
        console.error('❌ useGlobalLogStream: Failed to initialize:', error)
      }
    }

    initializeService()

    // Cleanup on unmount (app shutdown)
    return () => {
      console.log('🧹 useGlobalLogStream: Cleaning up global log service')
      logStreamService.shutdown()
    }
  }, [dispatch])
}
