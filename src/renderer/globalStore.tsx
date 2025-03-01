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
  setCircuitInUse,
  setMyIp,
};

// 1. State
interface GlobalState {
  commandOutput: string;
  torActive: boolean;
  relayActive: boolean;
  circuits: Array<Circuit> | null;
  circuitInUse: Circuit;
  relays: Array<Relay>;
  myIp: string;
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
  circuitInUse: {} as Circuit,
  relays: [],
  myIp: "166.205.90.66",
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
    setCircuitInUse: (state, action: PayloadAction<Circuit>) => {
      state.circuitInUse = action.payload;
    },
    setRelays: (state, action: PayloadAction<Array<Relay>>) => {
      state.relays = action.payload;
    },
    setMyIp: (state, action: PayloadAction<string>) => {
      state.myIp = action.payload;
    }
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
  setMyIp,
} = globalStore.actions;

type Circuit = {
  id: number;
  relays: Relay[]; // usually 3 Guard, Middle, Exit
};

type Relay = {
  bandwidth: number;
  contact: string | null;
  fingerprint: string;
  hop: number;
  ip: string;
  nickname: string;
  payment_bip353: string | null;
  payment_bolt11_lightning_address: string | null;
  payment_bolt11_lnurl: string | null;
  payment_bolt12_offer: string | null;
  payment_handshake_fee: number | null;
  payment_handshake_fee_payhash: string;
  payment_handshake_fee_preimage: string;
  payment_id_hashes_10: string[];
  payment_interval_rounds: number;
  payment_interval_seconds: number;
  payment_rate_msats: number;
  port: number;
  relay_tag: string;
};
