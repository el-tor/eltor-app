import { isTauri } from '../utils/platform'
import config from '../config'
import type {
  FetchWalletBalanceResponseType,
  FetchChannelInfoResponseType,
} from '../features/wallet/Wallet'

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

// Backend API response types that match the Rust structs
interface WalletBalanceResponse {
  total_balance_sats: number
  confirmed_balance_sats: number
  unconfirmed_balance_sats: number
  locked_balance_sats?: number
}

interface NodeInfoResponse {
  node_id: string
  alias?: string
  version?: string
  block_height?: number
  num_peers?: number
  num_channels?: number
  balance_sats?: number
  node_type: string
}

interface TransactionResponse {
  payment_hash: string
  created_at: number
  amount_msats: number
  preimage?: string
  payer_note?: string
}

interface ListTransactionsResponse {
  transactions: TransactionResponse[]
}

class WalletApiService {
  // Get wallet balance
  async getWalletBalance(): Promise<FetchWalletBalanceResponseType> {
    if (isTauri()) {
      await loadTauriAPIs()
      try {
        const data: WalletBalanceResponse = await tauriInvoke(
          'get_wallet_balance',
        )
        // Convert backend response to frontend format
        return { balance: data.total_balance_sats }
      } catch (error) {
        throw new Error(`Failed to get wallet balance: ${error}`)
      }
    } else {
      const response = await fetch(`${WEB_API_BASE}/api/wallet/balance`)

      if (!response.ok) {
        const error = await response.text()
        throw new Error(error)
      }

      const data: WalletBalanceResponse = await response.json()
      // Convert backend response to frontend format
      return { balance: data.total_balance_sats }
    }
  }

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

  // Get channel info (for now, use balance as send amount and derive receive from node info)
  async getChannelInfo(): Promise<FetchChannelInfoResponseType> {
    try {
      // For now, use the balance as the send amount and set receive to 0
      // In the future, this could be enhanced to get actual channel capacity info
      const balanceResponse = await this.getWalletBalance()
      const nodeInfo = await this.getNodeInfo()

      return {
        send: balanceResponse.balance,
        receive: nodeInfo.balance_sats || 0, // This could be enhanced to show actual receive capacity
      }
    } catch (error) {
      console.warn('Failed to get channel info, using defaults:', error)
      return { send: 0, receive: 0 }
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
