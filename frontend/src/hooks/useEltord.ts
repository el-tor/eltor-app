import { useEffect, useState } from 'react'
import { apiService } from '../services/apiService'
import type { FoundationClientStatus } from '../services/apiService'
import { isTauri } from '../utils/platform'
import { useDispatch, useSelector } from '../hooks'
import { setClientActive, setRelayActive, addLogClient, addLogRelay } from '../globalStore'

interface UseEltordOptions {
  mode: 'client' | 'relay' | 'both'
}

export function useEltord(options: UseEltordOptions) {
  const [isRunning, setIsRunning] = useState(false)
  const [loading, setLoading] = useState(false)
  const [isLoadingDeactivate, setLoadingDeactivate] = useState(false)
  const [foundationStatus, setFoundationStatus] = useState<FoundationClientStatus | null>(null)
  const dispatch = useDispatch()
  const { clientActive, relayActive } = useSelector((state) => state.global)

  const { mode } = options

  // Check if this specific mode is currently active
  const isModeActive = mode === 'client' ? clientActive : relayActive
  // Check if any mode is running (both can run independently now)
  const isAnyModeRunning = clientActive || relayActive

  const checkStatus = async () => {
    try {
      const status = await apiService.getEltordStatus()
      setFoundationStatus(status.client_running ? status.foundation_client_status ?? null : null)
      
      // Update Redux state to match backend reality
      if (status.client_running !== clientActive) {
        dispatch(setClientActive(status.client_running))
        console.log(`📊 [useEltord] Synced clientActive to ${status.client_running}`)
      }
      
      if (status.relay_running !== relayActive) {
        dispatch(setRelayActive(status.relay_running))
        console.log(`📊 [useEltord] Synced relayActive to ${status.relay_running}`)
      }
      
      // Update local isRunning state based on this specific mode
      const modeRunning = mode === 'client' ? status.client_running : status.relay_running
      if (modeRunning !== isRunning) {
        setIsRunning(modeRunning)
        console.log(`🔄 [useEltord] Synced local isRunning to ${modeRunning} for ${mode}`)
      }
    } catch (error) {
      setFoundationStatus(previous => previous ? 'recovering' : null)
      console.error('Failed to check eltord status:', error)
    }
  }

  const activate = async (enableLogging?: boolean) => {
    console.log(`🚀 [useEltord] Starting activation for mode: ${mode}, enableLogging: ${enableLogging}`)
    setLoading(true)
    try {
      // Since processes can now run independently, we don't need to check
      // if another mode is running. Each mode can be activated independently.
      
      console.log(`📡 [useEltord] Calling apiService.activateEltord mode: ${mode}`)
      await apiService.activateEltord(mode, enableLogging)
      console.log(`✅ [useEltord] Successfully activated ${mode} mode`)
      
      setTimeout(() => {
        // Extra check after short delay to ensure state is synced
        checkStatus()
        setLoading(false)
      }, 2000)
      console.log(`📊 [useEltord] Status checked after activation`)
    } catch (error) {
      console.error(`❌ [useEltord] Failed to activate eltord (${mode}):`, error)
      setLoading(false)
      throw error
    } finally {
      console.log(`🏁 [useEltord] Finished activation attempt for ${mode}`)
    }
  }

  const deactivate = async () => {
    console.log(`🛑 [useEltord] Starting deactivation for mode: ${mode}`)
    setLoadingDeactivate(true)
    try {
      // Both Tauri and web mode now support mode-specific deactivation
      console.log(`📡 [useEltord] Calling apiService.deactivateEltord for ${mode}`)
      await apiService.deactivateEltord(mode)
      checkStatus()
      console.log(`📊 [useEltord] Status checked after deactivation`)
    } catch (error) {
      console.error(`❌ [useEltord] Failed to deactivate eltord (${mode}):`, error)
      
      // Handle the specific case where backend says "No eltord process is currently running"
      // This indicates the frontend state is out of sync with the backend
      const errorMessage = error instanceof Error ? error.message : String(error)
      if (errorMessage.includes(`No eltor ${mode} task is currently running`)) {
        console.log(`🔄 [useEltord] Backend says ${mode} not running, syncing frontend state`)
        
        // Check status from backend to sync state
        await checkStatus()
        
        // Don't re-throw the error since we've handled the state sync
        console.log(`✅ [useEltord] State synchronized for ${mode}`)
        return
      }
      
      throw error
    } finally {
      setLoadingDeactivate(false)
      console.log(`🏁 [useEltord] Finished deactivation attempt for ${mode}`)
    }
  }

  useEffect(() => {
    checkStatus()

    let cleanup: (() => void) | undefined

    // Set up event listeners (Tauri only)
    apiService
      .listenToEvents((eventName, payload) => {
        switch (eventName) {
          case 'eltord-activated':
            setIsRunning(true)
            // Update the state for this specific mode
            if (mode === 'client') {
              dispatch(setClientActive(true))
            } else {
              dispatch(setRelayActive(true))
            }
            // Dispatch log entry to appropriate mode
            if (mode === 'client') {
              dispatch(addLogClient({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: 'Client eltord process activated',
                source: 'system'
              }))
            } else {
              dispatch(addLogRelay({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: 'Relay eltord process activated',
                source: 'system'
              }))
            }
            break
          case 'eltord-deactivated':
            setIsRunning(false)
            if (mode !== 'relay') setFoundationStatus(null)
            // Clear state for this specific mode
            if (mode === 'client') {
              dispatch(setClientActive(false))
            } else {
              dispatch(setRelayActive(false))
            }
            // Dispatch log entry to appropriate mode
            if (mode === 'client') {
              dispatch(addLogClient({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: 'Client eltord process deactivated',
                source: 'system'
              }))
            } else {
              dispatch(addLogRelay({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: 'Relay eltord process deactivated',
                source: 'system'
              }))
            }
            break
          case 'eltord-error':
            console.error('Eltord error:', payload)
            // Handle specific error cases that might indicate state desync
            const payloadStr = typeof payload === 'string' ? payload : String(payload)
            if (payloadStr.includes(`No eltord ${mode} process is currently running`)) {
              console.log(`🔄 [useEltord] Event indicates ${mode} not running, syncing state`)
              
              // Update Redux state to match backend reality
              if (mode === 'client') {
                dispatch(setClientActive(false))
              } else {
                dispatch(setRelayActive(false))
              }
              
              // Update local state
              setIsRunning(false)
            }
            break
        }
      })
      .then((unsubscribe) => {
        cleanup = unsubscribe
      })

    // Poll status every 5 seconds (both Tauri and web mode)
    // This ensures UI stays in sync even if events are missed
    const interval = setInterval(checkStatus, 5000)

    return () => {
      cleanup?.()
      clearInterval(interval)
    }
  }, [mode, dispatch])

  return { 
    isRunning: isModeActive, // This specific mode is running
    foundationStatus,
    isAnyModeRunning, // Any mode is running  
    loading, 
    isLoadingDeactivate,
    activate, 
    deactivate, 
    checkStatus 
  }
}
