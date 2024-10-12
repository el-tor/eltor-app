import { spawn } from "child_process";

export { openTerminalWithCommand };

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
