import { createSlice, createAsyncThunk } from "@reduxjs/toolkit";
import type {
  FetchWalletBalanceResponseType,
  WalletProviderType,
  FetchChannelInfoResponseType,
} from  "./Wallet";
import type { PayloadAction, SerializedError } from "@reduxjs/toolkit";
import { walletApiService } from "../../services/walletApiService";

const defaultWallet = "Phoenixd";

export {
  type WalletState,
  walletStore,
  walletReducer,
  setDefaultWallet,
  fetchWalletBalance,
  fetchChannelInfo,
  getBolt12Offer,
  fetchTransactions,
};

// 1. State
interface WalletState {
  balance: number;
  defaultWallet: WalletProviderType;
  requestState: RequestState;
  loading: boolean;
  error: SerializedError | null;
  channelInfo: FetchChannelInfoResponseType;
  bolt12Offer: string;
  transactions: Array<any>; // Todo type Transaction
}

type RequestState = "idle" | "pending" | "fulfilled" | "rejected";

const initialState: WalletState = {
  balance: 0,
  defaultWallet: "None",
  requestState: "idle",
  loading: false,
  error: null,
  channelInfo: {
    send: 0,
    receive: 0,
  },
  bolt12Offer: "",
  transactions: [],
}; // satisfies WalletState as WalletState

// 2. Slice and Reducers
const walletStore = createSlice({
  name: "wallet",
  initialState,
  reducers: {
    setDefaultWallet: (state, action: PayloadAction<WalletProviderType>) => {
      state.defaultWallet = action.payload;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchWalletBalance.pending, (state) => {
        state.requestState = "pending";
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchWalletBalance.fulfilled, (state, action) => {
        state.balance = action.payload.balance;
        state.requestState = "fulfilled";
        state.loading = false;
        state.error = null;
      })
      .addCase(fetchWalletBalance.rejected, (state, action) => {
        state.requestState = "rejected";
        state.loading = false;
        state.error = action.error;
      })

      .addCase(fetchChannelInfo.pending, (state) => {
        state.requestState = "pending";
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchChannelInfo.fulfilled, (state, action) => {
        state.channelInfo.receive = action.payload.receive;
        state.channelInfo.send = state.balance;
        state.requestState = "fulfilled";
        state.loading = false;
        state.error = null;
      })
      .addCase(fetchChannelInfo.rejected, (state, action) => {
        state.requestState = "rejected";
        state.loading = false;
        state.error = action.error;
      })

      .addCase(getBolt12Offer.pending, (state) => {
        state.requestState = "pending";
        state.loading = true;
        state.error = null;
      })
      .addCase(getBolt12Offer.fulfilled, (state, action) => {
        state.bolt12Offer = action.payload.payment_request;
        state.requestState = "fulfilled";
        state.loading = false;
        state.error = null;
      })
      .addCase(getBolt12Offer.rejected, (state, action) => {
        state.requestState = "rejected";
        state.loading = false;
        state.error = action.error;
      })

      .addCase(fetchTransactions.pending, (state) => {
        state.requestState = "pending";
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchTransactions.fulfilled, (state, action) => {
        state.transactions = action.payload;
        state.requestState = "fulfilled";
        state.loading = false;
        state.error = null;
      })
      .addCase(fetchTransactions.rejected, (state, action) => {
        state.requestState = "rejected";
        state.loading = false;
        state.error = action.error;
      });
  },
});

const walletReducer = walletStore.reducer;
// Action creators are generated for each case reducer function
const { setDefaultWallet } = walletStore.actions;

// 3. Async Thunks
const fetchWalletBalance = createAsyncThunk<
  FetchWalletBalanceResponseType,
  string
>("wallet/fetchWalletBalance", async (name, { rejectWithValue }) => {
  try {
    const data = await walletApiService.getWalletBalance();
    return data;
  } catch (error) {
    return rejectWithValue(error);
  }
});

const fetchChannelInfo = createAsyncThunk<FetchChannelInfoResponseType, string>(
  "wallet/fetchChannelInfo",
  async (name, { rejectWithValue }) => {
    try {
      const data = await walletApiService.getChannelInfo();
      return data;
    } catch (error) {
      return rejectWithValue(error);
    }
  }
);

const fetchTransactions = createAsyncThunk<Array<any>, string>( // Todo type Transaction
  "wallet/fetchTransactions",
  async (name, { rejectWithValue }) => {
    try {
      const txns = await walletApiService.getTransactions();
      return txns;
    } catch (error) {
      return rejectWithValue(error);
    }
  }
);

const getBolt12Offer = createAsyncThunk<any, string, { state: { wallet: WalletState } }>(
  "wallet/getBolt12Offer",
  async (name, { rejectWithValue, getState }) => {
    try {
      const state = getState().wallet;
      if (state.bolt12Offer) {
        console.info("Using cached Bolt12 offer");
        return { payment_request: state.bolt12Offer };
      }
      const offer = await walletApiService.getBolt12Offer();
      return offer;
    } catch (error) {
      return rejectWithValue(error);
    }
  }
);
