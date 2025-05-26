// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts

const { contextBridge } = require("electron");
import { electronEvents } from "../main/eventEmitter";

contextBridge.exposeInMainWorld("electronEvents", electronEvents);

declare global {
  interface Window {
    electronEvents: typeof electronEvents;
  }
}
