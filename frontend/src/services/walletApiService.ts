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

// Web API base URL - now configurable
const WEB_API_BASE = config.API_BASE_URL

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
      const response = await fetch(`${WEB_API_BASE}/api/wallet/info`)

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
      const response = await fetch(`${WEB_API_BASE}/api/wallet/transactions`)

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
      const response = await fetch(`${WEB_API_BASE}/api/wallet/offer`, {
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
}

export const walletApiService = new WalletApiService()
