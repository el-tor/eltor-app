import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";

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
};

// 1. State
interface GlobalState {
  commandOutput: string;
  torActive: boolean;
  relayActive: boolean;
  circuits: Array<Circuit>;
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
    setCircuits: (state, action: PayloadAction<Array<Circuit>>) => {
      state.circuits = action.payload;
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
} = globalStore.actions;

type Circuit = {
  id: string;
  fingerprint: string;
  relayFingerprints: string[];
  relayIps: string[];
  status: string; // "unknown" | ??
  idleTimeout: number | null;
  predictiveBuildTime: number | null;
  createdAt: string | null;
  expiresAt: string | null;
  isExpired: boolean;
};

type Relay = {
  fingerprint: string;
  preimage: string;
  payhash: string;
  lastPaymentStatus: string; // "accepted" | "denied"
  expiresAt: string;
};
