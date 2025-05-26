import { spawn } from "child_process";
import { BrowserWindow } from "electron";
import { openTerminalWithCommand } from "./utils";
import { ElectronEventsType } from "main/eventEmitter";
import { Circuit } from "renderer/globalStore";

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
      // startCircuitBuildWatcher(mainWindow);
      // torBrowserProcess.on("close", (code) => {
      //   console.log(`Tor Browser opened with code ${code}`);
      // });

      // 1. start el tor proxy 127.0.0.1:9099
      spawn("bash", ["cd ~/eltor/chutney/tor-proxy/eltor && ./tor -f torrc"], {
        stdio: "pipe",
      });

      // 2. start tor proxy 127.0.0.1:9098
      spawn("bash", ["cd ~/eltor/chutney/tor-proxy/tor && ./tor -f torrc"], {
        stdio: "pipe",
      });

      // 3. start haproxy on port 127.0.0.1:1080
      spawn(
        "bash",
        ["cd ~/eltor/chutney/tor-proxy/tor && haproxy -f haproxy.cfg"],
        {
          stdio: "pipe",
        }
      );
    });
  } else if (type === "relay") {
    openTerminalWithCommand("");
  }
}

export function startTorCargo(
  type: "browser" | "relay",
  mainWindow: BrowserWindow
) {
  // TODO OS specific commands

  if (type === "browser") {
    const cargoProcess = spawn(
      "bash",
      ["-c", 'cd ~/code/eltord && ARGS="eltord client -f torrc.client.prod -pw password1234_" cargo run'],
      {
        stdio: "pipe",
      }
    );

    let output = "";
    let lastCircuitId = 0;
    cargoProcess?.stdout?.on("data", (data) => {
      output += data.toString();
      console.log(data.toString());
      lastCircuitId = handleEvent(output, mainWindow, lastCircuitId);
      mainWindow.webContents.send(ElectronEventsType.onTorStdout, output);
    });
    cargoProcess?.stderr?.on("data", (data) => {
      output += data.toString();
      console.log(data.toString());
      mainWindow.webContents.send(ElectronEventsType.onTorStdout, output);
    });
  } else if (type === "relay") {
    openTerminalWithCommand("");
  }
}

export function handleEvent(event: string, mainWindow: BrowserWindow, lastCircuitId: number) {
  // parse the event data EVENT:{event_type, data}:ENDEVENT
  const eventData = event.match(/EVENT:(.*):ENDEVENT/);
  if (eventData && eventData[1]) {
    try {
      const parsedData = JSON.parse(eventData[1]);
      switch (parsedData.event) {
        case "CIRCUIT_BUILT":
          const circuit: Circuit = {
            id: parsedData.circuit_id,
            relays: parsedData.relays,

          }
          console.log(JSON.stringify(circuit, null, 2));
          if (lastCircuitId !== circuit.id) {
            mainWindow.webContents.send(ElectronEventsType.onPayCircuit, {
              circuits: [circuit],
              circuitInUse: circuit,
            });
            mainWindow.webContents.send(ElectronEventsType.onCircuitRenew, {
              circuit,
            });
          }
          lastCircuitId = circuit.id;
          break;
        default:
          console.warn(`Unhandled event type: ${parsedData.event}`);
          break;
      }
    } catch (error) {
      console.error("Failed to parse event data:", error);
    }
  }
  return lastCircuitId;
}
