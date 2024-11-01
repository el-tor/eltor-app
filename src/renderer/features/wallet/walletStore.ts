import { createSlice, createAsyncThunk } from "@reduxjs/toolkit";
import {
  IWallet,
  type FetchWalletBalanceResponseType,
  type WalletProviderType,
  type FetchChannelInfoResponseType,
} from "renderer/drivers/IWallet";
import type { PayloadAction, SerializedError } from "@reduxjs/toolkit";
import { dynamicWalletImport } from "renderer/utils";

const defaultWallet = "Phoenix";
const walletApi = dynamicWalletImport<IWallet>(defaultWallet);

export {
  type WalletState,
  walletStore,
  walletReducer,
  setDefaultWallet,
  fetchWalletBalance,
  fetchChannelInfo,
  getBolt12Offer,
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
        state.bolt12Offer = action.payload;
        state.requestState = "fulfilled";
        state.loading = false;
        state.error = null;
      })
      .addCase(getBolt12Offer.rejected, (state, action) => {
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
    const data = await walletApi.fetchWalletBalance();
    return data;
  } catch (error) {
    return rejectWithValue(error);
  }
});

const fetchChannelInfo = createAsyncThunk<FetchChannelInfoResponseType, string>(
  "wallet/fetchChannelInfo",
  async (name, { rejectWithValue }) => {
    try {
      const data = await walletApi.fetchChannelInfo("");
      return data;
    } catch (error) {
      return rejectWithValue(error);
    }
  }
);

const getBolt12Offer = createAsyncThunk<string, string>(
  "wallet/getBolt12Offer",
  async (name, { rejectWithValue }) => {
    try {
      const offer = await walletApi.getBolt12Offer();
      return offer;
    } catch (error) {
      return rejectWithValue(error);
    }
  }
);
