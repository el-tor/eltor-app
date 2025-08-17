// frontend/src/utils/platform.ts
export const isTauri = () => {
  const result = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
  console.log('🔍 [isTauri] Platform check result:', result)
  if (typeof window !== 'undefined') {
    console.log('🔍 [isTauri] Available window properties:', Object.keys(window).filter(k => k.includes('TAURI')))
  }
  return result
}

export const isWeb = () => !isTauri()

// Debug function to check what's available
export const debugPlatform = () => {
  if (typeof window === 'undefined') {
    console.log('🔍 Platform: SSR/Node environment')
    return
  }
  
  console.log('🔍 Platform Detection:')
  console.log('- window.__TAURI__:', '__TAURI__' in window)
  console.log('- window.__TAURI_INTERNALS__:', '__TAURI_INTERNALS__' in window)
  console.log('- window.__TAURI_METADATA__:', '__TAURI_METADATA__' in window)
  console.log('- User Agent:', navigator.userAgent)
  console.log('- Is Tauri:', isTauri())
}