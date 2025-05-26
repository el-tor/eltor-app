import { useEffect, useState } from 'react'
import { apiService } from '../services/apiService'
import { isTauri } from '../utils/platform'

export function useEltord() {
  const [isRunning, setIsRunning] = useState(false)
  const [loading, setLoading] = useState(false)

  const checkStatus = async () => {
    try {
      const status = await apiService.getEltordStatus()
      setIsRunning(status.running)
    } catch (error) {
      console.error('Failed to check eltord status:', error)
    }
  }

  const activate = async () => {
    setLoading(true)
    try {
      await apiService.activateEltord()
      await checkStatus()
    } catch (error) {
      console.error('Failed to activate eltord:', error)
      throw error
    } finally {
      setLoading(false)
    }
  }

  const deactivate = async () => {
    setLoading(true)
    try {
      await apiService.deactivateEltord()
      await checkStatus()
    } catch (error) {
      console.error('Failed to deactivate eltord:', error)
      throw error
    } finally {
      setLoading(false)
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
            break
          case 'eltord-deactivated':
            setIsRunning(false)
            break
          case 'eltord-error':
            console.error('Eltord error:', payload)
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
  }, [])

  return { isRunning, loading, activate, deactivate, checkStatus }
}
