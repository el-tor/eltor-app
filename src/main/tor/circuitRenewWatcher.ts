import { BrowserWindow } from "electron";
import fs from "fs";
import { ElectronEventsType } from "main/eventEmitter";
import readline from "readline";

export interface RelayRenew {
  fingerprint: string;
  nickname: string;
  identity: string;
  ip: string;
}

export interface CircuitRenew {
  id: string;
  idleTimeout: number;
  predictiveBuildTime: number;
  createdAt: string;
  relays: RelayRenew[];
  lastUsed: string | null;
}

export async function circuitRenewWatcher(
  logFilePath: string,
  mainWindow: BrowserWindow | null
) {
  const circuits = new Map<string, CircuitRenew>();
  let fileSize = 0;
  let circuitClosed = false;

  // Function to process new lines
  const processNewLines = async (newLines: string[]) => {
    const circuitClosedPattern =
      /connection_edge_reached_eof: conn \(fd \d+\) reached eof. Closing./;
    const circuitCreationPattern =
      /origin_circuit_new: Circuit (\d+) chose an idle timeout of (\d+) based on (\d+) seconds/;
    const circuitExtensionPattern =
      /extend_info_from_node: Including Ed25519 ID for \$([A-F0-9]{40})~([^\s]+) \[([^\]]+)\] at ([\d.]+)/;
    const circuitUsagePattern =
      /connection_edge_process_relay_cell_not_open: 'connected' received for circid (\d+) streamid (\d+) after (\d+) seconds/;

    for (const line of newLines) {
      if (circuitClosedPattern.test(line)) {
        circuitClosed = true;
      }

      let match;

      // Check for circuit creation
      match = line.match(circuitCreationPattern);
      if (match) {
        const circuitId = match[1];
        const idleTimeout = match[2];
        const predictiveBuildTime = match[3];
        circuits.set(circuitId, {
          id: circuitId,
          idleTimeout: parseInt(idleTimeout),
          predictiveBuildTime: parseInt(predictiveBuildTime),
          createdAt: new Date().toISOString(),
          relays: [],
          lastUsed: null,
        });

        if (circuitClosed) {
          const newCircuit = circuits.get(circuitId);
          console.log("Alert: A new circuit was created:", newCircuit);
          circuitClosed = false; // Reset the flag after alerting
        }

        // Process the next 50 lines to find relay information
        await processRelayInfo(
          newLines.slice(newLines.indexOf(line) + 1),
          circuitId
        );
        continue;
      }

      // Check for circuit usage
      match = line.match(circuitUsagePattern);
      if (match) {
        const circuitId = match[1];
        const circuit = circuits.get(circuitId);
        if (circuit) {
          circuit.lastUsed = new Date().toISOString();
        }
        continue;
      }
    }
  };

  // Function to process relay information
  const processRelayInfo = async (lines: string[], circuitId: string) => {
    const circuitExtensionPattern =
      /extend_info_from_node: Including Ed25519 ID for \$([A-F0-9]{40})~([^\s]+) \[([^\]]+)\] at ([\d.]+)/;
    const circuit = circuits.get(circuitId);

    for (const line of lines.slice(0, 50)) {
      const match = line.match(circuitExtensionPattern);
      if (match && circuit) {
        const fingerprint = match[1];
        const nickname = match[2];
        const identity = match[3];
        const ip = match[4];
        circuit.relays.push({ fingerprint, nickname, identity, ip });
      }
    }

    console.log("Updated circuit with relays:", circuit);
    if (mainWindow && circuit?.relays.length && circuit?.relays.length  >= 3) {
      mainWindow.webContents.send(ElectronEventsType.onCircuitRenew, {
        circuit,
      });
    }
  };

  // Function to read new lines from the file
  const readNewLines = async () => {
    const stats = fs.statSync(logFilePath);
    if (stats.size > fileSize) {
      const stream = fs.createReadStream(logFilePath, {
        start: fileSize,
        end: stats.size,
      });

      const rl = readline.createInterface({
        input: stream,
        crlfDelay: Infinity,
      });

      const newLines: string[] = [];
      for await (const line of rl) {
        newLines.push(line);
      }

      await processNewLines(newLines);
      fileSize = stats.size;
    }
  };

  // Watch the file for changes
  fs.watch(logFilePath, async (eventType) => {
    if (eventType === "change") {
      await readNewLines();
    }
  });

  // Initial read to set the file size
  const initialStats = fs.statSync(logFilePath);
  fileSize = initialStats.size;
}

export function startCircuitRenewWatcher() {
  const logFilePath = process.env.TOR_BROWSER_INFO_LOG_FILE_PATH;
  if (!logFilePath) {
    console.error(
      "TOR_BROWSER_INFO_LOG_FILE_PATH is not set in the environment variables."
    );
    return;
  }

  circuitRenewWatcher(logFilePath, null)
    .then(() => {
      console.log("Started watching for circuit renew events.");
    })
    .catch((error) => {
      console.error("Error:", error);
    });
}
