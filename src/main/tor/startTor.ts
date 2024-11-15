import { spawn } from "child_process";
import { BrowserWindow } from "electron";
import { openTerminalWithCommand } from "./utils";
import { startCircuitBuildWatcher } from "./circuitBuildWatcher";
import { ElectronEventsType } from "main/eventEmitter";

export function startTor(type: "browser" | "relay", mainWindow: BrowserWindow) {
  // TODO OS specific commands

  if (type === "browser") {
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
      mainWindow.webContents.send(ElectronEventsType.onTorStdout, output);
    });
    eltorDownloadProcess?.stderr?.on("data", (data) => {
      output += data.toString();
      console.log(data.toString());
      mainWindow.webContents.send(ElectronEventsType.onTorStdout, output);
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
      // const torBrowserProcess = spawn("open", [
      //   "/Applications/Tor Browser.app",
      // ]);
      startCircuitBuildWatcher(mainWindow);
      // torBrowserProcess.on("close", (code) => {
      //   console.log(`Tor Browser opened with code ${code}`);
      // });
    });
  } else if (type === "relay") {
    openTerminalWithCommand("");
  }
}
