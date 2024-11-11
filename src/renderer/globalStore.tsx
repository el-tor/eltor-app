import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";
import { CircuitRenew } from "main/tor/circuitRenewWatcher";

export {
  type GlobalState,
  type Circuit,
  type Relay,
  globalStore,
  globalReducer,
  setCommandOutput,
  setTorActive,
  setRelayActive,
  setCircuits,
  setRelays,
  setCircuitInUse,
};

// 1. State
interface GlobalState {
  commandOutput: string;
  torActive: boolean;
  relayActive: boolean;
  circuits: Array<Circuit> | null;
  circuitInUse: CircuitRenew;
  relays: Array<Relay>;
}

const initialState: GlobalState = {
  commandOutput: `
Waiting to connect...

%@@@@@@@@@@@@@%%%@@@@%%%%%%%%%@@@@@@@@@@@@@@@@%%%%%@@@@@@%%%%%%%@@@@@@@@@@%%%%%%
@#***********#%%%****%%%%%%%@%********###**#%%%%@@%#++*#%@@%%%%%#*******##%@@%%%
@=:::::::::::-%@#::::#@%%%%@@#:::::::::::::-%@%@%+::::::-*%@%%%@+:::::::::-+%@%%
@=:::::::::::-@@#::::#@%%%%@@%:::::::::::::-%@@*::::::::::-#@%%@+:::::::::::-%%%
@=:::::::::::-@@#::::#@%%%%%@%:::::::::::::-@@+:::::::::::::#@%@+::::::::::::+@%
@=::::======-=%@#::::#@%%%%%@#-----::::----=@#::::::::::::::-%@@+::::-=--:::::#%
@=:::=@@@@@@@@@@#::::#@%%%%%%%@@@@#::::#@@@%%-::::-#%%%+:::::+@@+:::-%@@%+::::*@
@=:::=@@@@@@@@@@#::::#@%%%%@@@@%%@#::::#@%%@#::::-%@@@@@#::::-%@+:::-%@@@%::::+@
@=::::------*@@@#::::#@%%@%###%@%@#::::#@%%@+::::#@%%%%@@=::::%@+:::-@@@@*::::*@
@=::::::::::+@@@#::::#@%@%=--:+@%@#::::#@%%@+::::%@%%%%%@*::::#@+::::+++=:::::#@
@+::::::::::+@@@#::::#@%%%****#%%@#::::#@%%@+::::%@%%%%%@*::::#@+::::::::::::=%%
@+::::::::::+@@@#::::#@%%%@@@@@%%@#::::#@%%@+::::#@%%%%%@=::::%@+:::::::::::-%@%
@+:::=%%%%%%%@@@#::::#@%%%%@@%%%%@*::::#@%%@#::::-%@%%%@*::::-%@+::::::::---%@%%
@+:--=@%%%%@%%%@#:--:*%%%%%%%%%%%@*:--:#@%%%%-:--:-#%%%+:---:*@@+:---+*----:#@%%
@+:-----------+@#:-----------%@%%@*:--:#@%%%@#:--------:---:-%%@+:---%@+:----%%%
@+-----------:+@#:----------:#@%%@*:--:#@%%%%@*:----------:-%@%%+:---%@%----:+@%
@+------------+@#-----------:#%%%@*:--:#@%%%%%@*-:-------:=%@%%%+:---%%@#:----#@
@=:::--------:+@#::---:::--::#@%%@*::::#@%%%%%%@%+-------*%@%%%%+:::-%%%@*:--:-%
@############*#@%**#*********%%%%%#****%%%%%%%%%%@%#***#%@%%%%%%#****%%%%%**##*#
%@@@@@@@@@@@@@@%%@@%@@@@@%%@@%%%%%%@@@@%%%%%%%%%%%%@@@@%%%%%%%%%%@@@@%%%%%@@@%%%
`,
  torActive: false,
  relayActive: false,
  circuits: [],
  circuitInUse: {} as CircuitRenew,
  relays: [],
};

// 2. Slice and Reducers
const globalStore = createSlice({
  name: "global",
  initialState,
  reducers: {
    setCommandOutput: (state, action: PayloadAction<string>) => {
      state.commandOutput = action.payload;
    },
    setTorActive: (state, action: PayloadAction<boolean>) => {
      state.torActive = action.payload;
    },
    setRelayActive: (state, action: PayloadAction<boolean>) => {
      state.relayActive = action.payload;
    },
    setCircuits: (state, action: PayloadAction<Array<Circuit> | null>) => {
      state.circuits = action.payload;
    },
    setCircuitInUse: (state, action: PayloadAction<CircuitRenew>) => {
      state.circuitInUse = action.payload;
    },
    setRelays: (state, action: PayloadAction<Array<Relay>>) => {
      state.relays = action.payload;
    },
  },
  extraReducers: (builder) => {},
});

const globalReducer = globalStore.reducer;
// Action creators are generated for each case reducer function
const {
  setCommandOutput,
  setTorActive,
  setRelayActive,
  setCircuits,
  setRelays,
  setCircuitInUse,
} = globalStore.actions;

type Circuit = {
  id?: string | null;
  fingerprint?: string;
  relayFingerprints?: string[];
  relayIps?: string[];
  status?: string; // "unknown" | ??
  idleTimeout?: number | null | string;
  predictiveBuildTime?: number | null | string;
  createdAt?: string | null;
  expiresAt?: string | null;
  isExpired?: boolean;
  circuitFingerprint?: string | null;
  lastUsed?: string | null;
};

type Relay = {
  fingerprint?: string;
  preimage?: string;
  payhash?: string;
  lastPaymentStatus?: string; // "accepted" | "denied"
  expiresAt?: string;
};
