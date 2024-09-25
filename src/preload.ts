// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts

import { contextBridge, ipcRenderer } from 'electron';

// Expose IPC methods to the renderer process
contextBridge.exposeInMainWorld("electron", {
  navigateToWallet: () => ipcRenderer.send("navigate-to-wallet"),
  onNavigateToWallet: (callback: () => void) => {
    ipcRenderer.on("navigate-to-wallet", callback);
  },
});