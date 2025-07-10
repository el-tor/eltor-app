import { createSlice, createAsyncThunk } from '@reduxjs/toolkit'
import type {
  WalletProviderType,
  FetchChannelInfoResponseType,
} from './Wallet'
import type { PayloadAction, SerializedError } from '@reduxjs/toolkit'
import { 
  TransactionResponse, 
  walletApiService, 
  type NodeInfoResponse,
  type LightningConfigRequest,
  type DeleteLightningConfigRequest,
  type LightningConfigResponse,
  type MessageResponse
} from '../../services/walletApiService'

export {
  type WalletState,
  walletStore,
  walletReducer,
  setDefaultWallet,
  setClickedWallet,
  getBolt12Offer,
  fetchTransactions,
  fetchNodeInfo,
  fetchLightningConfigs,
  upsertLightningConfig,
  deleteLightningConfig,
  clearLightningConfigError,
}

// 1. State
interface WalletState {
  send: number
  receive: number
  defaultWallet: WalletProviderType
  clickedWallet?: WalletProviderType
  requestState: RequestState
  loading: boolean
  error: SerializedError | null
  channelInfo: FetchChannelInfoResponseType
  bolt12Offer: string
  transactions: Array<TransactionResponse>
  
  // Lightning configuration state
  lightningConfigs: Array<LightningConfigResponse>
  lightningConfigsLoading: boolean
  lightningConfigsError: string | null
  defaultLightningConfig: LightningConfigResponse | null
}

type RequestState = 'idle' | 'pending' | 'fulfilled' | 'rejected'

const initialState: WalletState = {
  send: 0,
  receive: 0,
  defaultWallet: 'none',
  clickedWallet: 'none',
  requestState: 'idle',
  loading: false,
  error: null,
  channelInfo: {
    send: 0,
    receive: 0,
  },
  bolt12Offer: '',
  transactions: [],
  
  // Lightning configuration initial state
  lightningConfigs: [],
  lightningConfigsLoading: false,
  lightningConfigsError: null,
  defaultLightningConfig: null,
} // satisfies WalletState as WalletState

// 2. Slice and Reducers
const walletStore = createSlice({
  name: 'wallet',
  initialState,
  reducers: {
    setDefaultWallet: (state, action: PayloadAction<WalletProviderType>) => {
      state.defaultWallet = action.payload
    },
    setClickedWallet: (state, action: PayloadAction<WalletProviderType>) => {
      state.clickedWallet = action.payload
    },
    clearLightningConfigError: (state) => {
      state.lightningConfigsError = null
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(getBolt12Offer.pending, (state) => {
        state.requestState = 'pending'
        state.loading = true
        state.error = null
      })
      .addCase(getBolt12Offer.fulfilled, (state, action) => {
        state.bolt12Offer = action.payload.payment_request
        state.requestState = 'fulfilled'
        state.loading = false
        state.error = null
      })
      .addCase(getBolt12Offer.rejected, (state, action) => {
        state.requestState = 'rejected'
        state.loading = false
        state.error = action.error
      })

      .addCase(fetchTransactions.pending, (state) => {
        state.requestState = 'pending'
        state.loading = true
        state.error = null
      })
      .addCase(fetchTransactions.fulfilled, (state, action) => {
        state.transactions = action.payload
        state.requestState = 'fulfilled'
        state.loading = false
        state.error = null
      })
      .addCase(fetchTransactions.rejected, (state, action) => {
        state.requestState = 'rejected'
        state.loading = false
        state.error = action.error
      })

      .addCase(fetchNodeInfo.pending, (state, action) => {
        state.requestState = 'pending'
        state.loading = true
        state.error = null
      })
      .addCase(fetchNodeInfo.fulfilled, (state, action) => {
        state.defaultWallet = action.payload.node_type || 'none'
        state.send = Math.round(action.payload.send_balance_msat / 1000)
        state.receive = Math.round(action.payload.receive_balance_msat / 1000)
        state.requestState = 'fulfilled'
        state.loading = false
        state.error = null
      })
      .addCase(fetchNodeInfo.rejected, (state, action) => {
        state.requestState = 'rejected'
        state.loading = false
        state.error = action.error
      })

      // Lightning Config Cases
      .addCase(fetchLightningConfigs.pending, (state) => {
        state.lightningConfigsLoading = true
        state.lightningConfigsError = null
      })
      .addCase(fetchLightningConfigs.fulfilled, (state, action) => {
        state.lightningConfigs = action.payload
        state.defaultLightningConfig = action.payload.find((config: LightningConfigResponse) => config.is_default) || null
        state.lightningConfigsLoading = false
        state.lightningConfigsError = null
      })
      .addCase(fetchLightningConfigs.rejected, (state, action) => {
        state.lightningConfigsLoading = false
        state.lightningConfigsError = action.error.message || 'Failed to fetch lightning configs'
      })

      .addCase(upsertLightningConfig.pending, (state) => {
        state.lightningConfigsLoading = true
        state.lightningConfigsError = null
      })
      .addCase(upsertLightningConfig.fulfilled, (state, action) => {
        state.lightningConfigsLoading = false
        state.lightningConfigsError = null
        // Note: We'll dispatch fetchLightningConfigs after this to refresh the list
      })
      .addCase(upsertLightningConfig.rejected, (state, action) => {
        state.lightningConfigsLoading = false
        state.lightningConfigsError = action.error.message || 'Failed to upsert lightning config'
      })

      .addCase(deleteLightningConfig.pending, (state) => {
        state.lightningConfigsLoading = true
        state.lightningConfigsError = null
      })
      .addCase(deleteLightningConfig.fulfilled, (state, action) => {
        state.lightningConfigsLoading = false
        state.lightningConfigsError = null
        // Note: We'll dispatch fetchLightningConfigs after this to refresh the list
      })
      .addCase(deleteLightningConfig.rejected, (state, action) => {
        state.lightningConfigsLoading = false
        state.lightningConfigsError = action.error.message || 'Failed to delete lightning config'
      })
  },
})

const walletReducer = walletStore.reducer
// Action creators are generated for each case reducer function
const { setDefaultWallet, setClickedWallet, clearLightningConfigError } = walletStore.actions


const fetchTransactions = createAsyncThunk<Array<any>, string>( // Todo type Transaction
  'wallet/fetchTransactions',
  async (name, { rejectWithValue }) => {
    try {
      const txns = await walletApiService.getTransactions()
      return txns
    } catch (error) {
      return rejectWithValue(error)
    }
  },
)

const getBolt12Offer = createAsyncThunk<
  any,
  string,
  { state: { wallet: WalletState } }
>('wallet/getBolt12Offer', async (name, { rejectWithValue, getState }) => {
  try {
    const offer = await walletApiService.getBolt12Offer()
    return offer
  } catch (error) {
    return rejectWithValue(error)
  }
})

const fetchNodeInfo = createAsyncThunk<
  any,
  string,
  { state: { wallet: WalletState } }
>('wallet/getNodeInfo', async (name, { rejectWithValue, getState }) => {
  try {
    const info = await walletApiService.getNodeInfo()
    return info
  } catch (error) {
    return rejectWithValue(error)
  }
})

// Lightning Config Async Thunks
const fetchLightningConfigs = createAsyncThunk<
  Array<LightningConfigResponse>,
  void,
  { state: { wallet: WalletState } }
>('wallet/fetchLightningConfigs', async (_, { rejectWithValue }) => {
  try {
    const configs = await walletApiService.listLightningConfigs()
    return configs
  } catch (error) {
    return rejectWithValue(error)
  }
})

const upsertLightningConfig = createAsyncThunk<
  MessageResponse,
  LightningConfigRequest,
  { state: { wallet: WalletState } }
>('wallet/upsertLightningConfig', async (config, { rejectWithValue, dispatch }) => {
  try {
    const result = await walletApiService.upsertLightningConfig(config)
    // Refresh the configs list after successful upsert
    dispatch(fetchLightningConfigs())
    return result
  } catch (error) {
    return rejectWithValue(error)
  }
})

const deleteLightningConfig = createAsyncThunk<
  MessageResponse,
  DeleteLightningConfigRequest,
  { state: { wallet: WalletState } }
>('wallet/deleteLightningConfig', async (config, { rejectWithValue, dispatch }) => {
  try {
    const result = await walletApiService.deleteLightningConfig(config)
    // Refresh the configs list after successful deletion
    dispatch(fetchLightningConfigs())
    return result
  } catch (error) {
    return rejectWithValue(error)
  }
})
