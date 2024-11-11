// this watches the tor info log for circuit build events.
// if a circuit is ready to build we emit an event to pay the relays
import fs from "fs";
import readline from "readline";
import dotenv from "dotenv";
import { BrowserWindow } from "electron";
import { ElectronEventsType } from "main/eventEmitter";
import { type Circuit } from "renderer/globalStore";
import { circuitRenewWatcher } from "./circuitRenewWatcher";

dotenv.config();

export function startCircuitBuildWatcher(mainWindow: BrowserWindow) {
  const logFilePath = process.env.TOR_BROWSER_INFO_LOG_FILE_PATH;
  if (!logFilePath) {
    console.error(
      "TOR_BROWSER_INFO_LOG_FILE_PATH is not set in the environment variables."
    );
    return;
  }
  // TODO: kill interval on tor deactivate
  // setInterval(() => {
  parseCircuits(logFilePath)
    .then(({ circuits, circuitInUse }) => {
      const filteredCircuits = circuits.filter(
        (c) =>
          c.relayFingerprints.length === 3 &&
          !c.isExpired &&
          c.relayIps.length >= 3
      );
      console.log(JSON.stringify(filteredCircuits, null, 2));
      mainWindow.webContents.send(ElectronEventsType.onPayCircuit, {
        circuits: filteredCircuits,
        circuitInUse,
      });
    })
    .catch((error) => {
      console.error("Error:", error);
    });
  // }, 10000);

  circuitRenewWatcher(logFilePath, mainWindow);
}

async function parseCircuits(logFilePath: string) {
  const circuits = new Map<string, Circuit>();
  const fileStream = fs.createReadStream(logFilePath);
  const rl = readline.createInterface({
    input: fileStream,
    crlfDelay: Infinity,
  });

  const circuitPattern = /Circuit (\d+)/;
  const fingerprintPattern = /\$([A-F0-9]{40})/;
  const circuitFingerprintPattern = /built \$([A-F0-9]{40})/i;
  const statusPattern = /Circuit (\d+) (BUILT|EXTENDED|FAILED|CLOSED)/i;
  const originCircuitPattern =
    /origin_circuit_new: Circuit (\d+) chose an idle timeout of (\d+) based on (\d+) seconds/;
  const relayIpPattern = /(\d{1,3}\.){3}\d{1,3}/;
  const circuitUsagePattern =
    /channelpadding_send_padding_cell_for_callback: Sending netflow keepalive on (\d+) to \[scrubbed\] \(\[scrubbed\]\) after (\d+) ms. Delta (\d+)ms/;

  for await (const line of rl) {
    // Check for circuit ID, fingerprint and status
    const circuitMatch = line.match(circuitPattern);
    const statusMatch = line.match(statusPattern);
    const originMatch = line.match(originCircuitPattern);

    if (circuitMatch) {
      const circuitId = circuitMatch[1];
      const circuitFingerprintMatch = line.match(circuitFingerprintPattern);

      if (circuitId && !circuits.has(circuitId)) {
        circuits.set(circuitId, {
          id: circuitId,
          circuitFingerprint: circuitFingerprintMatch
            ? circuitFingerprintMatch[1]
            : null,
          relayFingerprints: [],
          status: "unknown",
          idleTimeout: null,
          predictiveBuildTime: null,
          createdAt: null,
          relayIps: [],
          lastUsed: null,
        });
      }

      // Update status if found
      if (statusMatch && statusMatch[1] === circuitId) {
        const circuit = circuits.get(circuitId);
        if (circuit) {
          circuit.status = statusMatch[2];
        }
      }

      // Update timeout info if found
      if (originMatch && originMatch[1] === circuitId) {
        const circuit = circuits.get(circuitId);
        if (circuit) {
          circuit.idleTimeout = originMatch[2] && parseInt(originMatch[2]);
          circuit.predictiveBuildTime =
            originMatch[3] && parseInt(originMatch[3]);
          circuit.createdAt = new Date().toISOString();
        }
      }
    }

    // Check for relay fingerprints
    const fingerprintMatch = line.match(fingerprintPattern);
    if (fingerprintMatch && circuits.size > 0) {
      const lastCircuitId = Array.from(circuits.keys()).pop();
      const circuit = circuits.get(lastCircuitId);
      if (
        circuit &&
        !circuit?.relayFingerprints?.includes(fingerprintMatch?.[1])
      ) {
        circuit.relayFingerprints?.push(fingerprintMatch[1]);
      }
    }

    // Check for relay IPs
    const relayIpMatch = line.match(relayIpPattern);
    if (relayIpMatch && circuits.size > 0) {
      const lastCircuitId = Array.from(circuits.keys()).pop();
      const circuit = circuits.get(lastCircuitId);
      if (circuit && !circuit.relayIps) {
        circuit.relayIps = [];
      }
      if (circuit && !circuit.relayIps.includes(relayIpMatch[0])) {
        circuit.relayIps.push(relayIpMatch[0]);
      }
    }

    // Check for circuit usage
    const usageMatch = line.match(circuitUsagePattern);
    if (usageMatch) {
      const circuitId = usageMatch[1];
      const circuit = circuits.get(circuitId);
      if (circuit) {
        circuit.lastUsed = new Date().toISOString();
      }
    }
  }

  // Find the most recently used circuit
  let mostRecentCircuit: Circuit | null = null;
  let mostRecentTime = 0;
  circuits.forEach((circuit) => {
    if (circuit.lastUsed) {
      const lastUsedTime = new Date(circuit.lastUsed).getTime();
      if (lastUsedTime > mostRecentTime) {
        mostRecentTime = lastUsedTime;
        mostRecentCircuit = circuit;
      }
    }
  });

  const circuitInUse = mostRecentCircuit;
  console.log("Circuit in use:", circuitInUse);

  return {
    circuits: Array.from(circuits.values()).map((circuit: Circuit) => {
      if (circuit.idleTimeout && circuit.createdAt) {
        const createdDate = new Date(circuit.createdAt);
        const expiresAt = new Date(
          createdDate.getTime() + circuit.idleTimeout * 1000
        );
        const now = new Date();

        return {
          ...circuit,
          expiresAt: expiresAt.toISOString(),
          isExpired: now > expiresAt,
        };
      }
      return circuit;
    }),
    circuitInUse,
  };
}
