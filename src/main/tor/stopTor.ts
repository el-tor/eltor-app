import { spawn } from "child_process";
import { BrowserWindow } from "electron";
import { openTerminalWithCommand } from "./utils";

export function stopTor(type: "browser" | "relay", mainWindow: BrowserWindow) {
  // TODO OS specific commands

  if (type === "browser") {
    // Spawn a new shell to run the complex bash command
    const eltorDownloadProcess = spawn(
      "bash",
      [
        "-c",
        "curl -L https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/uninstall.sh | bash",
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
      let stopCommand: string;
      let stopArgs: string[];

      if (process.platform === "win32") {
        stopCommand = "taskkill";
        stopArgs = ["/F", "/IM", "tor.exe"];
      } else if (process.platform === "darwin") {
        stopCommand = "pkill";
        stopArgs = ["-f", "Tor Browser"];
      } else {
        stopCommand = "pkill";
        stopArgs = ["-f", "tor"];
      }

      const stopTorBrowserProcess = spawn(stopCommand, stopArgs);
      stopTorBrowserProcess.on("close", (code) => {
        console.log(`Tor Browser stopped with code ${code}`);
      });
    });
  } else if (type === "relay") {
    openTerminalWithCommand("");
  }
}
