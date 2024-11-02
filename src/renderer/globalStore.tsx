import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";


export {
  type GlobalState,
  globalStore,
  globalReducer,
  setCommandOutput,
  setTorActive,
  setRelayActive,
};

// 1. State
interface GlobalState {
  commandOutput: string;
  torActive: boolean;
  relayActive: boolean;
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
  },
  extraReducers: (builder) => {
    
  },
});

const globalReducer = globalStore.reducer;
// Action creators are generated for each case reducer function
const { setCommandOutput, setTorActive, setRelayActive } = globalStore.actions;


