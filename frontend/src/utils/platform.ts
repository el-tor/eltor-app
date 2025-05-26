// frontend/src/utils/platform.ts
export const isTauri = () => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
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