import { spawn } from 'child_process'
import { BrowserWindow } from 'electron'
import { openTerminalWithCommand } from './utils'
import { ElectronEventsType } from 'main/eventEmitter'

export function stopTor(type: 'browser' | 'relay', mainWindow: BrowserWindow) {
  // TODO OS specific commands

  if (type === 'browser') {
    // Spawn a new shell to run the complex bash command
    const eltorDownloadProcess = spawn(
      'bash',
      [
        '-c',
        'curl -L https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/uninstall.sh | bash',
      ],
      {
        stdio: 'pipe',
      },
    )

    let output = ''
    eltorDownloadProcess?.stdout?.on('data', (data) => {
      output += data.toString()
      console.log(data.toString())
      mainWindow.webContents.send(ElectronEventsType.onTorStdout, output)
    })
    eltorDownloadProcess?.stderr?.on('data', (data) => {
      output += data.toString()
      console.log(data.toString())
      mainWindow.webContents.send(ElectronEventsType.onTorStdout, output)
    })
    eltorDownloadProcess.on('close', (code) => {
      // resolve(output);
    })
    eltorDownloadProcess.on('error', (err) => {
      // reject(err);
    })

    eltorDownloadProcess.on('close', (code) => {
      console.log(`Eltor install script finished with code ${code}`)
      let stopCommand: string
      let stopArgs: string[]

      // TODO fix when cargo is not being used and the eltord daemon is used
      if (process.platform === 'win32') {
        stopCommand = 'taskkill'
        stopArgs = ['/F', '/IM', 'cargo.exe']
      } else if (process.platform === 'darwin') {
        stopCommand = 'pkill'
        stopArgs = ['cargo']
      } else {
        stopCommand = 'pkill'
        stopArgs = ['cargo']
      }

      const stopTorBrowserProcess = spawn(stopCommand, stopArgs)
      stopTorBrowserProcess.on('close', (code) => {
        console.log(`Tor Browser stopped with code ${code}`)
      })
    })
  } else if (type === 'relay') {
    openTerminalWithCommand('')
  }
}

export async function stopTorCargo(
  type: 'browser' | 'relay',
  mainWindow: BrowserWindow,
) {
  if (type === 'browser') {
    try {
      const pid = await findEltorProcess(mainWindow)

      if (!pid) {
        console.log('No eltor process found to terminate')
        return
      }

      let stopCommand: string
      let stopArgs: string[]

      if (process.platform === 'win32') {
        stopCommand = 'taskkill'
        stopArgs = ['/F', '/PID', pid.toString()]
      } else {
        stopCommand = 'kill'
        stopArgs = ['-9', pid.toString()]
      }

      const stopProcess = spawn(stopCommand, stopArgs, {
        stdio: 'pipe',
      })

      stopProcess.on('close', (code) => {
        console.log(`Eltor process (PID: ${pid}) stopped with code ${code}`)
        mainWindow.webContents.send(
          ElectronEventsType.onTorStdout,
          `Eltor process (PID: ${pid}) stopped with code ${code}`,
        )
      })
    } catch (error) {
      console.error('Error stopping eltor process:', error)
      mainWindow.webContents.send(
        ElectronEventsType.onTorStdout,
        `Error stopping eltor process: ${error}`,
      )
    }
  } else if (type === 'relay') {
    openTerminalWithCommand('')
  }
}

export function findEltorProcess(
  mainWindow: BrowserWindow,
): Promise<number | null> {
  return new Promise((resolve, reject) => {
    let command: string
    let args: string[]

    if (process.platform === 'win32') {
      // Windows
      command = 'tasklist'
      args = ['/FI', 'IMAGENAME eq eltor.exe', '/FO', 'CSV', '/NH']
    } else if (process.platform === 'darwin') {
      // macOS
      command = 'pgrep'
      args = ['eltor']
    } else {
      // Linux
      command = 'pgrep'
      args = ['eltor']
    }

    const findProcess = spawn(command, args, { stdio: 'pipe' })

    let output = ''
    findProcess.stdout?.on('data', (data) => {
      output += data.toString()
    })

    findProcess.on('close', (code) => {
      if (code !== 0) {
        console.log(`Process finder exited with code ${code}`)
        resolve(null) // No process found
        return
      }

      // Parse the output to get PID
      let pid: number | null = null

      if (process.platform === 'win32') {
        // Windows CSV output parsing
        const match = output.match(/"([^"]+)","(\d+)",/)
        if (match && match[2]) {
          pid = parseInt(match[2], 10)
        }
      } else {
        // macOS/Linux output parsing
        const firstLine = output.trim().split('\n')[0]
        if (firstLine && !isNaN(parseInt(firstLine, 10))) {
          pid = parseInt(firstLine, 10)
        }
      }

      console.log(`Found eltor process with PID: ${pid}`)
      resolve(pid)
    })

    findProcess.on('error', (err) => {
      console.error('Error finding process:', err)
      reject(err)
    })
  })
}
