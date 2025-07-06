import { useEffect, useState } from 'react'
import { apiService } from '../services/apiService'
import { isTauri } from '../utils/platform'
import { useDispatch, useSelector } from '../hooks'
import { setClientActive, setRelayActive, addLogClient, addLogRelay } from '../globalStore'

interface UseEltordOptions {
  torrcFile?: string
  mode?: 'client' | 'relay'
}

export function useEltord(torrcFileOrOptions?: string | UseEltordOptions) {
  const [isRunning, setIsRunning] = useState(false)
  const [loading, setLoading] = useState(false)
  const dispatch = useDispatch()
  const { clientActive, relayActive } = useSelector((state) => state.global)

  // Handle both old and new API signatures
  const options: UseEltordOptions = typeof torrcFileOrOptions === 'string' 
    ? { torrcFile: torrcFileOrOptions, mode: 'client' }
    : torrcFileOrOptions || { mode: 'client' }

  const { torrcFile, mode = 'client' } = options

  // Check if this specific mode is currently active
  const isModeActive = mode === 'client' ? clientActive : relayActive
  // Check if any mode is running (both can run independently now)
  const isAnyModeRunning = clientActive || relayActive

  const checkStatus = async () => {
    try {
      const status = await apiService.getEltordStatus()
      // The global status only tells us about the client process (legacy)
      // We'll rely on our Redux state to track individual modes
      // Only update local isRunning state, don't touch Redux states here
      setIsRunning(isModeActive)
    } catch (error) {
      console.error('Failed to check eltord status:', error)
    }
  }

  const activate = async () => {
    console.log(`🚀 [useEltord] Starting activation for mode: ${mode}`)
    setLoading(true)
    try {
      // Since processes can now run independently, we don't need to check
      // if another mode is running. Each mode can be activated independently.
      
      console.log(`📡 [useEltord] Calling apiService.activateEltord with torrcFile: ${torrcFile}, mode: ${mode}`)
      await apiService.activateEltord(torrcFile, mode)
      console.log(`✅ [useEltord] Successfully activated ${mode} mode`)
      
      // Update Redux state to reflect this mode is now active
      if (mode === 'client') {
        dispatch(setClientActive(true))
        console.log(`📊 [useEltord] Set clientActive to true`)
      } else {
        dispatch(setRelayActive(true))
        console.log(`📊 [useEltord] Set relayActive to true`)
      }
      
      // Update local isRunning state immediately
      setIsRunning(true)
      console.log(`🔄 [useEltord] Set local isRunning to true for ${mode}`)
    } catch (error) {
      console.error(`❌ [useEltord] Failed to activate eltord (${mode}):`, error)
      throw error
    } finally {
      setLoading(false)
      console.log(`🏁 [useEltord] Finished activation attempt for ${mode}`)
    }
  }

  const deactivate = async () => {
    console.log(`🛑 [useEltord] Starting deactivation for mode: ${mode}`)
    setLoading(true)
    try {
      // Both Tauri and web mode now support mode-specific deactivation
      console.log(`📡 [useEltord] Calling apiService.deactivateEltordWithMode for ${mode}`)
      await apiService.deactivateEltordWithMode(mode)
      
      // Clear only this mode's active state
      if (mode === 'client') {
        dispatch(setClientActive(false))
        console.log(`📊 [useEltord] Set clientActive to false`)
      } else {
        dispatch(setRelayActive(false))
        console.log(`📊 [useEltord] Set relayActive to false`)
      }
      
      // Update local isRunning state immediately
      setIsRunning(false)
      console.log(`🔄 [useEltord] Set local isRunning to false for ${mode}`)
    } catch (error) {
      console.error(`❌ [useEltord] Failed to deactivate eltord (${mode}):`, error)
      
      // Handle the specific case where backend says "No eltord process is currently running"
      // This indicates the frontend state is out of sync with the backend
      const errorMessage = error instanceof Error ? error.message : String(error)
      if (errorMessage.includes(`No eltord ${mode} process is currently running`)) {
        console.log(`🔄 [useEltord] Backend says ${mode} not running, syncing frontend state`)
        
        // Update Redux state to match backend reality
        if (mode === 'client') {
          dispatch(setClientActive(false))
          console.log(`📊 [useEltord] Synced clientActive to false`)
        } else {
          dispatch(setRelayActive(false))
          console.log(`📊 [useEltord] Synced relayActive to false`)
        }
        
        // Update local isRunning state
        setIsRunning(false)
        console.log(`🔄 [useEltord] Synced local isRunning to false for ${mode}`)
        
        // Don't re-throw the error since we've handled the state sync
        console.log(`✅ [useEltord] State synchronized for ${mode}`)
        return
      }
      
      throw error
    } finally {
      setLoading(false)
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

    // For web mode, poll status every 5 seconds
    let interval: NodeJS.Timeout | undefined
    if (!isTauri()) {
      interval = setInterval(checkStatus, 5000)
    }

    return () => {
      cleanup?.()
      if (interval) clearInterval(interval)
    }
  }, [mode, dispatch])

  return { 
    isRunning: isModeActive, // This specific mode is running
    isAnyModeRunning, // Any mode is running  
    loading, 
    activate, 
    deactivate, 
    checkStatus 
  }
}
