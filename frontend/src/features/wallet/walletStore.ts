import { createSlice, createAsyncThunk } from '@reduxjs/toolkit'
import type {
  WalletProviderType,
  FetchChannelInfoResponseType,
} from './Wallet'
import type { PayloadAction, SerializedError } from '@reduxjs/toolkit'
import { TransactionResponse, walletApiService, type NodeInfoResponse } from '../../services/walletApiService'

const defaultWallet = 'phoenixd'

export {
  type WalletState,
  walletStore,
  walletReducer,
  setDefaultWallet,
  getBolt12Offer,
  fetchTransactions,
  fetchNodeInfo,
}

// 1. State
interface WalletState {
  send: number
  receive: number
  defaultWallet: WalletProviderType
  requestState: RequestState
  loading: boolean
  error: SerializedError | null
  channelInfo: FetchChannelInfoResponseType
  bolt12Offer: string
  transactions: Array<TransactionResponse>
}

type RequestState = 'idle' | 'pending' | 'fulfilled' | 'rejected'

const initialState: WalletState = {
  send: 0,
  receive: 0,
  defaultWallet: 'none',
  requestState: 'idle',
  loading: false,
  error: null,
  channelInfo: {
    send: 0,
    receive: 0,
  },
  bolt12Offer: '',
  transactions: [],
} // satisfies WalletState as WalletState

// 2. Slice and Reducers
const walletStore = createSlice({
  name: 'wallet',
  initialState,
  reducers: {
    setDefaultWallet: (state, action: PayloadAction<WalletProviderType>) => {
      state.defaultWallet = action.payload
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
  },
})

const walletReducer = walletStore.reducer
// Action creators are generated for each case reducer function
const { setDefaultWallet } = walletStore.actions


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
