// this watches the tor info log for circuit build events.
// if a circuit is ready to build we emit an event to pay the relays
import fs from "fs";
import readline from "readline";
import dotenv from "dotenv";
import { BrowserWindow } from "electron";
import { ElectronEventsType } from "main/eventEmitter";

dotenv.config();

export function startCircuitBuildWatcher(mainWindow: BrowserWindow) {
  const logFilePath = process.env.TOR_BROWSER_INFO_LOG_FILE_PATH;
  if (!logFilePath) {
    console.error(
      "TOR_BROWSER_INFO_LOG_FILE_PATH is not set in the environment variables."
    );
    return;
  }

  let fileSize = 0;
  const processedCircuits = new Set<string>();

  const readNewLines = () => {
    const stats = fs.statSync(logFilePath);
    if (stats.size > fileSize) {
      const stream = fs.createReadStream(logFilePath, {
        start: fileSize,
        end: stats.size,
      });
      const rl = readline.createInterface({
        input: stream,
        output: process.stdout,
        terminal: false,
      });

      const fingerprints: string[] = [];
      let selectedCircuit: string | null = null;

      rl.on("line", (line) => {
        if (line.includes("extend_info_from_node")) {
          const match = line.match(/\$([A-F0-9]+)~/);
          if (match && match[1]) {
            fingerprints.push(match[1]);
          }
        }

        if (line.includes("circuit_handle_first_hop")) {
          selectedCircuit = line;
        }

        if (
          line.includes("connection_or_set_identity_digest") &&
          selectedCircuit
        ) {
          const match = line.match(/([A-F0-9]{40})/);
          if (match && match[1]) {
            const circuitId = match[1];
            if (!processedCircuits.has(circuitId)) {
              processedCircuits.add(circuitId);
              if (fingerprints.length === 3) {
                console.log(`Selected circuit: ${selectedCircuit}`);
                console.log(
                  `Fingerprints for hops 1, 2, and 3: ${fingerprints.join(
                    ", "
                  )}`
                );
                mainWindow.webContents.send(ElectronEventsType.onPayCircuit, fingerprints);
                fingerprints.length = 0; // Reset for the next circuit
                selectedCircuit = null; // Reset for the next circuit
              }
            }
          }
        }

        if (
          line.includes("circuit_mark_for_close") ||
          line.includes("circuit_expire_building")
        ) {
          // console.log(`Circuit rotation detected: ${line}`);
        }
      });

      rl.on("close", () => {
        fileSize = stats.size;
      });
    }
  };

  fs.watch(logFilePath, (eventType) => {
    if (eventType === "change") {
      readNewLines();
    }
  });

  // Initial read to catch up with the current file content
  readNewLines();
}
