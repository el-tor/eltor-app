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
    return {
      BACKEND_PORT: envConfig.BACKEND_PORT || defaultConfig.BACKEND_PORT,
      BACKEND_URL: envConfig.BACKEND_URL || defaultConfig.BACKEND_URL,
      API_BASE_URL: `${envConfig.BACKEND_URL || defaultConfig.BACKEND_URL}:${envConfig.BACKEND_PORT || defaultConfig.BACKEND_PORT}`
    }
  }

  // Fallback to defaults
  return defaultConfig
}

export const config = getConfig()
export default config
