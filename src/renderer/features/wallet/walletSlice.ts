import { createSlice, createAsyncThunk } from "@reduxjs/toolkit";
import {
  IWallet,
  type FetchWalletBalanceResponseType,
  type WalletProviderType,
} from "renderer/drivers/IWallet";
import type { PayloadAction, SerializedError } from "@reduxjs/toolkit";
import { dynamicWalletImport } from "renderer/utils";

const defaultWallet = "Phoenix"; // TODO: pull from redux or localStorage
const walletApi = dynamicWalletImport<IWallet>(defaultWallet);

export {
  type WalletState,
  walletSlice,
  walletReducer,
  setDefaultWallet,
  fetchWalletBalance,
}

// 1. State
interface WalletState {
  balance: number;
  defaultWallet: WalletProviderType;
  requestState: RequestState;
  loading: boolean;
  error: SerializedError | null;
}

type RequestState = "idle" | "pending" | "fulfilled" | "rejected";

const initialState: WalletState = {
  balance: 0,
  defaultWallet: "None",
  requestState: "idle",
  loading: false,
  error: null
}; // satisfies WalletState as WalletState

// 2. Slice and Reducers
const walletSlice = createSlice({
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
        state.requestState = 'pending';
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchWalletBalance.fulfilled, (state, action) => {
        state.balance = action.payload.balance;
        state.requestState = 'fulfilled';
        state.loading = false;
        state.error = null;
      })
      .addCase(fetchWalletBalance.rejected, (state, action) => {
        state.requestState = 'rejected';
        state.loading = false;
        state.error = action.error;
      });
  },
});

const walletReducer = walletSlice.reducer;
// Action creators are generated for each case reducer function
const { setDefaultWallet } = walletSlice.actions;

// 3. Async Thunks
const fetchWalletBalance = createAsyncThunk<
  FetchWalletBalanceResponseType,
  string
>("wallet/fetchWalletBalance", async (name, { rejectWithValue }) => {
  try {
    const data = await walletApi?.fetchWalletBalance();
    return data;
  } catch (error) {
    return rejectWithValue(error);
  }
});
