import {
  app,
  BrowserWindow,
  Tray,
  nativeImage,
  Menu,
  shell,
  ipcMain,
} from "electron";
import path from "node:path";
import { registerRoute } from "lib/electron-router-dom";
import fs from "fs";
import os from "os";
import { spawn } from "child_process";
require("dotenv").config({ path: path.join(app.getAppPath(), ".env") });
import installExtension, {
  REDUX_DEVTOOLS,
  REACT_DEVELOPER_TOOLS,
} from "electron-devtools-installer";
import { startTor } from "./tor/startTor";
import { stopTor } from "./tor/stopTor";

let tray: Tray;
let mainWindow: BrowserWindow;

async function createMainWindow() {
  const baseSrcPath = getSrcBasePath();
  console.log("baseSrcPath", baseSrcPath);
  const userDataPath = app.getPath("userData");
  console.log("userDataPath", userDataPath);

  mainWindow = new BrowserWindow({
    width: 1600,
    height: 1440,
    show: false,
    resizable: true,
    //alwaysOnTop: true,
    webPreferences: {
      preload: path.join(__dirname, "../preload/index.js"),
      contextIsolation: true,
      webSecurity: false,
    },
  });

  registerRoute({
    id: "main",
    browserWindow: mainWindow,
    htmlFile: path.join(__dirname, "../renderer/index.html"),
  });

  mainWindow.on("ready-to-show", mainWindow.show);
  // mainWindow.webContents.openDevTools();
}

app.whenReady().then(() => {
  createMainWindow();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createMainWindow();
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.whenReady().then(() => {
  installExtension(REDUX_DEVTOOLS)
    .then((name) => console.log(`Added Extension:  ${name}`))
    .catch((err) => console.log("An error occurred: ", err));
  installExtension(REACT_DEVELOPER_TOOLS)
    .then((name) => console.log(`Added Extension:  ${name}`))
    .catch((err) => console.log("An error occurred: ", err));

  createTrayMenu();

  if (os.platform() === "darwin") {
    // TODO: enable for windows and linux
    startWallet();
  }
});

function createTrayMenu() {
  const trayIconPath = getImagePath("eltor-logo-24.png"); // TODO fix path for diff OS
  const trayIcon = nativeImage.createFromPath(trayIconPath ?? "");
  trayIcon.setTemplateImage(true);
  if (trayIcon.isEmpty()) {
    console.error(`Failed to load tray icon from path: ${trayIconPath}`);
    return;
  }
  tray = new Tray(trayIcon);
  // Create a context menu
  const contextMenu = Menu.buildFromTemplate([
    {
      label: "Activate (Connect to El Tor)", //"Status: Active",
      enabled: true,
      id: "menu-activate-connect",
      click: () => {
        startTor("browser", mainWindow);
        trayNavigate("connect");
      },
    },
    {
      label: "Deactivate",
      enabled: false,
      id: "menu-deactivate-connect",
      click: () => {
        stopTor("browser", mainWindow);
        trayNavigate("deactivate-connect");
      },
    },
    { type: "separator" },
    {
      label: "Start Relay (Share bandwidth)", // "Status: Active Relay",
      enabled: true,
      id: "menu-activate-relay",
      click: () => {
        startTor("relay", mainWindow);
        trayNavigate("relay");
      },
    },
    {
      label: "Deactivate Relay",
      enabled: false,
      id: "menu-deactivate-relay",
      click: () => {
        trayNavigate("deactivate-relay");
      },
    },
    { type: "separator" },
    {
      label: "Manage Wallet",
      click: () => {
        trayNavigate("wallet");
      },
    },
    {
      label: "Manage Relay",
      click: () => {
        trayNavigate("relay");
      },
    },
    { type: "separator" },
    {
      label: "About El Tor",
      click: () => shell.openExternal("https://devpost.com/software/el-tor"),
    },
    { label: "Quit El Tor", click: () => app.quit() },
  ]);
  // Optional: Set a tooltip for the tray icon
  tray.setToolTip("El Tor");
  // Set the context menu for the tray
  tray.setContextMenu(contextMenu);

  ipcMain.handle("get-menu-item-state", (event, itemId) => {
    const menuItem = contextMenu.getMenuItemById(itemId);
    if (menuItem) {
      return menuItem.enabled;
    } else {
      return null; // or throw an error, depending on how you want to handle it
    }
  });

  ipcMain.on("set-menu-item-state", (event, itemId, state) => {
    const menuItem = contextMenu.getMenuItemById(itemId);
    if (menuItem) {
      menuItem.enabled = state;
      tray.setContextMenu(Menu.buildFromTemplate(contextMenu.items)); // Rebuild the menu to reflect changes
    } else {
      console.error(`Menu item with id ${itemId} not found`);
    }
  });
}

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and import them here.
function startWallet() {
  const phoenixd = path.join(
    app.getAppPath(),
    "src/renderer/drivers/wallets/phoenixd/phoenixd"
  ); // or daemon executable
  const phoenixdProcess = spawn(phoenixd, [], {});
  const phoenixdConfig = path.join(os.homedir(), ".phoenix/phoenix.conf");
  fs.readFile(phoenixdConfig, "utf8", (err, data) => {
    if (err) {
      // console.error("Error reading conf file:", err);
      return;
    }
    // console.log("Config file contents:", data);
  });
}

function trayNavigate(path: string) {
  console.log(`Clicked ${path}`);
  if (mainWindow) {
    // Restore if minimized
    if (mainWindow.isMinimized()) {
      mainWindow.restore();
    }
    // Focus the window (bring to foreground)
    mainWindow.focus();
  }
  mainWindow.webContents.send(`navigate-to-${path}`);
}

function getImagePath(filename: string) {
  let basePath;
  if (app.isPackaged) {
    // When packaged, use this path to navigate outside ASAR but within the app bundle
    basePath = path.join(getSrcBasePath(), "src", "renderer", "assets"); // 'app' directory or wherever your assets are placed outside ASAR
  } else {
    basePath = path.join(process.cwd(), "src", "renderer", "assets");
  }
  const imagePath = path.join(basePath, filename);

  if (fs.existsSync(imagePath)) {
    return imagePath;
  } else {
    console.error("Image not found at:", imagePath);
    return null;
  }
}

function getSrcBasePath() {
  let basePath;
  if (app.isPackaged) {
    // When packaged, use this path to navigate outside ASAR but within the app bundle
    basePath = path.join(app.getAppPath(), ".."); // 'app' directory or wherever your assets are placed outside ASAR
  } else {
    basePath = path.join(process.cwd(), "src");
  }
  return basePath;
}
