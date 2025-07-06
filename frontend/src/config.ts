// Frontend configuration that reads from environment variables
interface Config {
  BACKEND_PORT: string
  BACKEND_URL: string
  API_BASE_URL: string
}

// Default configuration
const defaultConfig: Config = {
  BACKEND_PORT: '5174',
  BACKEND_URL: `${window.location.protocol}//${window.location.hostname}`,
  API_BASE_URL: `${window.location.protocol}//${window.location.hostname}:5174`
}

// Read configuration from environment variables or use defaults
const getConfig = (): Config => {
  // In browser environment, check if config was injected
  if (typeof window !== 'undefined' && (window as any).__ENV_CONFIG__) {
    const envConfig = (window as any).__ENV_CONFIG__
    console.log('Frontend config - Received envConfig:', envConfig);
    
    // If API_BASE_URL is provided directly, use it
    // Otherwise, construct it from BACKEND_URL + BACKEND_PORT
    let apiBaseUrl = envConfig.API_BASE_URL;
    if (!apiBaseUrl) {
      const backendUrl = envConfig.BACKEND_URL || defaultConfig.BACKEND_URL;
      const backendPort = envConfig.BACKEND_PORT || defaultConfig.BACKEND_PORT;
      
      // If BACKEND_URL already contains a port or is a complete URL, use it as-is
      // Otherwise, append the port
      if (backendUrl.includes('://') && (backendUrl.includes(':80') || backendUrl.includes(':443') || backendUrl.match(/:\d+/))) {
        apiBaseUrl = backendUrl;
      } else if (backendUrl.includes('://')) {
        // URL without port, add the port
        apiBaseUrl = `${backendUrl}:${backendPort}`;
      } else {
        // Not a complete URL, construct it
        apiBaseUrl = `${backendUrl}:${backendPort}`;
      }
    }
    
    const result = {
      BACKEND_PORT: envConfig.BACKEND_PORT || defaultConfig.BACKEND_PORT,
      BACKEND_URL: envConfig.BACKEND_URL || defaultConfig.BACKEND_URL,
      API_BASE_URL: apiBaseUrl
    }
    
    console.log('Frontend config - Final config:', result);
    return result
  }

  // Fallback to defaults
  console.log('Frontend config - No envConfig found, using defaults:', defaultConfig);
  return defaultConfig
}

export const config = getConfig()
export default config
