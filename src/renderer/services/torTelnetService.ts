import fs from "fs";
import path from "path";
import { Telnet } from "telnet-client";

const { api } = window

type TorConfig = {
  torrcPath: string;
  controlHost: string;
  controlPort: number | string;
};

export async function torTelnetService(
  commands: Array<string>,
  torConfig?: TorConfig
) {
  // Path to the Tor control cookie
  const cookieFullPath = torConfig
    ? path.join(torConfig.torrcPath ?? "", "control_auth_cookie")
    : path.join(api.env.TOR_BROWSER_TORRC_PATH ?? "", "control_auth_cookie");

  // Read the cookie file
  const cookie = fs.readFileSync(cookieFullPath);
  // Convert the binary cookie to a hexadecimal string
  const hexCookie = cookie.toString("hex");

  // Telnet connection parameters
  const connection = new Telnet();
  const params = {
    host: torConfig?.controlHost
      ? torConfig?.controlHost
      : api.env.TOR_BROWSER_CONTROL_HOST || "127.0.0.1",
    port: torConfig?.controlPort
      ? torConfig.controlPort
      : api.env.TOR_BROWSER_CONTROL_PORT || 9051,
    negotiationMandatory: false,
    timeout: 1500,
  };

  try {
    await connection.connect(params);
    let formattedCommands = "";
    for (const cmd of commands) {
      formattedCommands = formattedCommands + cmd + "\r\n";
    }
    await connection.send(`AUTHENTICATE ${hexCookie}\r\n${formattedCommands}`);
    return true;
  } catch (error) {
    return false;
  }
}
