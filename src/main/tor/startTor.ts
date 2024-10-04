import { spawn } from "child_process";
import { BrowserWindow } from "electron";

export function startTor(type: "browser" | "relay", mainWindow: BrowserWindow) {
  // TODO OS specific commands

  if (type === "browser") {
    // ipcMain.handle("run-command", async () => {
    //   return new Promise((resolve, reject) => {
    // Spawn a new shell to run the complex bash command
    const eltorDownloadProcess = spawn(
      "bash",
      [
        "-c",
        "curl -L https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/install.sh | bash",
      ],
      {
        stdio: "pipe",
      }
    );

    let output = "";
    eltorDownloadProcess?.stdout?.on("data", (data) => {
      output += data.toString();
      console.log(data.toString());
      mainWindow.webContents.send("tor-stdout", output);
    });
    eltorDownloadProcess?.stderr?.on("data", (data) => {
      output += data.toString();
      console.log(data.toString());
      mainWindow.webContents.send("tor-stdout", output);
    });
    eltorDownloadProcess.on("close", (code) => {
      // resolve(output);
    });
    eltorDownloadProcess.on("error", (err) => {
      // reject(err);
    });

    eltorDownloadProcess.on("close", (code) => {
      console.log(`Eltor install script finished with code ${code}`);

      // After download, open Tor Browser
      const torBrowserProcess = spawn("open", [
        "/Applications/Tor Browser.app",
      ]);
      torBrowserProcess.on("close", (code) => {
        console.log(`Tor Browser opened with code ${code}`);
      });
    });
    //   });
    // });
  } else if (type === "relay") {
    openTerminalWithCommand("");
  }
}

function openTerminalWithCommand(command: string) {
  // Use osascript to open Terminal and run the command in a new window
  const script = `
    tell application "Terminal"
      do script "${command}"
      activate
    end tell
  `;
  spawn("osascript", ["-e", script]);
}