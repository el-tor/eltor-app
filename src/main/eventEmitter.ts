import { type Circuit } from "renderer/globalStore";
import { CircuitRenew } from "./tor/circuitRenewWatcher";
const { ipcRenderer } = require("electron");

export const electronEvents = {
  env: process.env, // TODO: only expose env vars from .env file

  // Terminal Commands
  onTorStdout: (callback: any) =>
    ipcRenderer.on(ElectronEventsType.onTorStdout, callback),

  // Connect Events
  navigateToConnect: () => ipcRenderer.send("navigate-to-connect"),
  onNavigateToConnect: (callback: () => void) => {
    ipcRenderer.on("navigate-to-connect", callback);
  },
  navigateToDeactivateConnect: () =>
    ipcRenderer.send("navigate-to-deactivate-connect"),
  onNavigateToDeactivateConnect: (callback: () => void) => {
    ipcRenderer.on("navigate-to-deactivate-connect", callback);
  },

  // Relay Events
  navigateToRelay: () => ipcRenderer.send("navigate-to-relay"),
  onNavigateToRelay: (callback: () => void) => {
    ipcRenderer.on("navigate-to-relay", callback);
  },
  navigateToDeactivateRelay: () =>
    ipcRenderer.send("navigate-to-deactivate-relay"),
  onNavigateToDeactivateRelay: (callback: () => void) => {
    ipcRenderer.on("navigate-to-deactivate-relay", callback);
  },

  // Wallet Events
  navigateToWallet: () => ipcRenderer.send("navigate-to-wallet"),
  onNavigateToWallet: (callback: () => void) => {
    ipcRenderer.on("navigate-to-wallet", callback);
  },

  // Menu Events
  menuActivateConnect: (callback: () => void) => {
    ipcRenderer.send("set-menu-item-state", "menu-activate-connect", false);
    ipcRenderer.send("set-menu-item-state", "menu-deactivate-connect", true);
  },
  menuDeactivateConnect: (callback: () => void) => {
    ipcRenderer.send("set-menu-item-state", "menu-deactivate-connect", false);
    ipcRenderer.send("set-menu-item-state", "menu-activate-connect", true);
  },
  menuActivateRelay: (callback: () => void) => {
    ipcRenderer.send("set-menu-item-state", "menu-activate-relay", false);
    ipcRenderer.send("set-menu-item-state", "menu-deactivate-relay", true);
  },
  menuDeactivateRelay: (callback: () => void) => {
    ipcRenderer.send("set-menu-item-state", "menu-deactivate-relay", false);
    ipcRenderer.send("set-menu-item-state", "menu-activate-relay", true);
  },

  // Tor Events
  onPayCircuit: (
    callback: (
      event: any,
      {
        circuits,
        circuitInUse,
      }: { circuits: Array<Circuit>; circuitInUse: Circuit }
    ) => void
  ) => ipcRenderer.on(ElectronEventsType.onPayCircuit, callback),

  onCircuitRenew: (
    callback: (event: any, { circuit }: { circuit: CircuitRenew }) => void
  ) => ipcRenderer.on(ElectronEventsType.onCircuitRenew, callback),
} as const;

export enum ElectronEventsType {
  onPayCircuit = "onPayCircuit",
  onCircuitRenew = "onCircuitRenew",
  onTorStdout = "onTorStdout",
}
