import { isTauri } from '../utils/platform'
import config from '../config'

// Tauri imports (only loaded when in Tauri mode)
let tauriInvoke: any = null

// Load Tauri APIs dynamically
const loadTauriAPIs = async () => {
  if (isTauri() && !tauriInvoke) {
    const coreModule = await import('@tauri-apps/api/core')
    tauriInvoke = coreModule.invoke
  }
}

// Web API base URL - construct from current location
const getApiBaseUrl = () => {
  // Just use the current location's protocol, hostname, and port
  // This works because the frontend is served from the same server as the API
  const currentUrl = `${window.location.protocol}//${window.location.host}`
  console.log(
    'walletApiService - Using current location as API base:',
    currentUrl,
  )
  return currentUrl
}

export interface NodeInfoResponse {
  alias: string
  color: string
  pubkey: string
  network: string
  block_height: number
  block_hash: string
  send_balance_msat: number
  receive_balance_msat: number
  fee_credit_balance_msat: number
  unsettled_send_balance_msat: number
  unsettled_receive_balance_msat: number
  pending_open_send_balance: number
  pending_open_receive_balance: number
  node_type?: string
}

export interface TransactionResponse {
  payment_hash: string
  created_at: number
  amount_msats: number
  preimage?: string
  payer_note?: string
  settled_at?: number
}

interface ListTransactionsResponse {
  transactions: TransactionResponse[]
}

// Lightning config interfaces
export interface LightningConfigRequest {
  node_type: 'phoenixd' | 'cln' | 'lnd'
  url: string
  password: string
  set_as_default: boolean
  is_embedded?: boolean // Indicates if this config is for an embedded Phoenix instance
}

export interface DeleteLightningConfigRequest {
  node_type: 'phoenixd' | 'cln' | 'lnd'
  url?: string // Optional - if not provided, deletes first match of node_type
}

export interface LightningConfigResponse {
  node_type: string
  url: string
  password_type: 'password' | 'rune' | 'macaroon'
  password: string // The actual credential value
  is_default: boolean
  is_embedded?: boolean // Indicates if this config is for an embedded Phoenix instance
}

export interface ListLightningConfigsResponse {
  configs: LightningConfigResponse[]
}

export interface MessageResponse {
  message: string
}

// Phoenix-specific interfaces
export interface PhoenixStartResponse {
  success: boolean
  message: string
  downloaded: boolean
  pid?: number
  url?: string
  password?: string
  is_running?: boolean
}

export interface PhoenixStopResponse {
  success: boolean
  message: string
  pid?: number
}

class WalletApiService {
  // Get node info (can be used to derive channel info)
  async getNodeInfo(): Promise<NodeInfoResponse> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        return await tauriInvoke('get_node_info')
      } catch (error) {
        throw new Error(`Failed to get node info: ${error}`)
      }
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/wallet/info`)

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      return await response.json()
    }
  }

  // Get transactions
  async getTransactions(): Promise<TransactionResponse[]> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        const data: ListTransactionsResponse = await tauriInvoke(
          'get_wallet_transactions',
        )
        return data.transactions
      } catch (error) {
        throw new Error(`Failed to get transactions: ${error}`)
      }
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/wallet/transactions`)

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      const data: ListTransactionsResponse = await response.json()
      return data.transactions
    }
  }

  // Initialize the Lightning node (Tauri mode only)
  async initializeLightningNode(): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        return await tauriInvoke('initialize_lightning_node')
      } catch (error) {
        throw new Error(`Failed to initialize Lightning node: ${error}`)
      }
    } else {
      // Web mode doesn't need initialization as the backend server handles this
      return 'Lightning node already running in backend'
    }
  }

  // Get BOLT12 offer (placeholder for now)
  async getBolt12Offer(): Promise<string> {
    if (isTauri()) {
      await loadTauriAPIs()
      return await tauriInvoke('get_offer')
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/wallet/offer`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({}),
      })
      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }
      return await response.json()
    }
  }

  // Upsert (create/update) lightning configuration
  async upsertLightningConfig(
    config: LightningConfigRequest,
  ): Promise<MessageResponse> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        return await tauriInvoke('upsert_lightning_config', { config })
      } catch (error) {
        throw new Error(`Failed to upsert lightning config: ${error}`)
      }
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/wallet/config`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(config),
      })

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      return await response.json()
    }
  }

  // Delete lightning configuration
  async deleteLightningConfig(
    config: DeleteLightningConfigRequest,
  ): Promise<MessageResponse> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        return await tauriInvoke('delete_lightning_config', { config })
      } catch (error) {
        throw new Error(`Failed to delete lightning config: ${error}`)
      }
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/wallet/config`, {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(config),
      })

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      return await response.json()
    }
  }

  // List all lightning configurations
  async listLightningConfigs(): Promise<LightningConfigResponse[]> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        const data: ListLightningConfigsResponse = await tauriInvoke(
          'list_lightning_configs',
        )
        return data.configs
      } catch (error) {
        throw new Error(`Failed to list lightning configs: ${error}`)
      }
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/wallet/configs`)

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      const data: ListLightningConfigsResponse = await response.json()
      return data.configs
    }
  }

  // Phoenix daemon management
  async startPhoenixDaemon(): Promise<PhoenixStartResponse> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        return await tauriInvoke('start_phoenix_daemon')
      } catch (error) {
        throw new Error(`Failed to start Phoenix daemon: ${error}`)
      }
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/phoenix/start`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      return await response.json()
    }
  }

  async stopPhoenixDaemon(): Promise<PhoenixStopResponse> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        return await tauriInvoke('stop_phoenix_daemon')
      } catch (error) {
        throw new Error(`Failed to stop Phoenix daemon: ${error}`)
      }
    } else {
      const response = await fetch(`${getApiBaseUrl()}/api/phoenix/stop`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      return await response.json()
    }
  }

  async detectPhoenixConfig(): Promise<PhoenixStartResponse> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        return await tauriInvoke('detect_phoenix_config')
      } catch (error) {
        throw new Error(`Failed to detect Phoenix config: ${error}`)
      }
    } else {
      // Use web API for detection in non-Tauri mode
      const response = await fetch(`${getApiBaseUrl()}/api/phoenix/detect-config`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      return await response.json()
    }
  }
}

export const walletApiService = new WalletApiService()
