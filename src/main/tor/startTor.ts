import { spawn } from "child_process";

export function startTor() {
  const eltorDownloadProcess = spawn('bash <(curl -L https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/install.sh)', [], {});
  const torBrowserProcess = spawn('open /Applications/Tor\ Browser.app', [], {});
}
