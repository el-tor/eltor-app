// this watches the tor info log for circuit build events.
// if a circuit is ready to build we emit an event to pay the relays 
import fs from 'fs';
import readline from 'readline';
import dotenv from 'dotenv';

dotenv.config();

export function startCircuitBuildWatcher() {
    const logFilePath = process.env.TOR_BROWSER_INFO_LOG_FILE_PATH;
    if (!logFilePath) {
        console.error('TOR_BROWSER_INFO_LOG_FILE_PATH is not set in the environment variables.');
        return;
    }

    const torInfoLog = fs.createReadStream(logFilePath);
    const rl = readline.createInterface({
        input: torInfoLog,
        output: process.stdout
    });

    const fingerprints: string[] = [];

    rl.on('line', (line) => {
        if (line.includes('extend_info_from_node')) {
            const match = line.match(/\$([A-F0-9]+)~/);
            if (match && match[1]) {
                fingerprints.push(match[1]);
            }
        }

        if (fingerprints.length === 3) {
            console.log('Fingerprints for hops 1, 2, and 3:', fingerprints);
            // eventEmitter.emit('circuitReadyToBuild', fingerprints);
            fingerprints.length = 0; // Reset for the next circuit
        }
    });

    rl.on('close', () => {
        console.log('Finished reading the log file.');
    });
}
