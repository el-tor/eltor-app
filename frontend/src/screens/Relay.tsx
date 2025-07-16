import {
  Stack,
  Title,
  Text,
  Group,
  Center,
  Loader,
  Badge,
  Button,
  Box,
} from '@mantine/core'
import { useEffect, useRef, useState } from 'react'
import { Circle } from '../components/Circle'
import CopyableTextBox from '../components/CopyableTextBox'
import { useDispatch, useSelector } from '../hooks'
import { isTauri } from '../utils/platform'
import { useEltord } from '../hooks/useEltord'
import { LogEntry } from '../services/apiService'
import LogViewer from '../components/LogViewer'
// @ts-ignore
import styles from '../globals.module.css'
import { clearLogsRelay } from '../globalStore'

export const Relay = () => {
  const { global, wallet } = useSelector((state) => state)
  const [loading, setLoading] = useState(false)
  const {
    isRunning: isRelayRunning,
    isAnyModeRunning,
    activate,
    deactivate,
  } = useEltord({
    torrcFile: 'torrc.relay',
    mode: 'relay',
  })
  const { logsRelay, relayActive, circuits, circuitInUse } = useSelector(
    (state) => state.global,
  )
  const dispatch = useDispatch()

  const preRef = useRef<HTMLPreElement>(null)
  // const [logs, setLogs] = useState<LogEntry[]>([])

  useEffect(() => {
    if (preRef.current) {
      preRef.current.scrollTop = preRef.current.scrollHeight
    }
  }, [logsRelay])

  // Add debug effect to log frontend state
  useEffect(() => {
    console.log('🔍 Relay Page - Current Redux State:')
    console.log('  - Client logs count:', global.logsClient?.length)
    console.log('  - Relay logs count:', global.logsRelay?.length)
    console.log('  - Client active:', global.clientActive)
    console.log('  - Relay active:', relayActive)
    console.log('  - Active mode:', global.activeMode)
  }, [
    global.logsClient.length,
    global.logsRelay.length,
    global.clientActive,
    relayActive,
    global.activeMode,
  ])

  return (
    <Stack>
      <Title order={2}>Relay</Title>
      {/* <Group w="100%">
        <Circle color={relayActive ? "lightgreen" : "#FF6347"} />
        <Title order={2}>{relayActive ? "Relay Active" : "Relay not active"}</Title>
        <Group ml="auto">
          <Center> {loading && <Loader size="sm" />}</Center>
        </Group>
      </Group> */}
      <Text>
        <b>1. Activate</b> the El Tor Relay
      </Text>
      <Group>
        <Button
          onClick={activate}
          disabled={isRelayRunning || loading}
          color="green"
          loading={loading}
        >
          {isRelayRunning ? 'Relay Active' : 'Activate Relay'}
        </Button>

        <Button
          onClick={async () => {
            try {
              await deactivate()
            } catch (error) {
              console.error('❌ [Relay] Deactivate error:', error)
              // Handle the case where backend says "No eltord relay process is currently running"
              // This means the frontend state is out of sync with backend
              if (
                error instanceof Error &&
                error.message.includes(
                  'No eltord relay process is currently running',
                )
              ) {
                console.log(
                  '🔄 [Relay] Backend says relay not running, syncing frontend state',
                )
                // The useEltord hook should handle state updates through the 'eltord-error' event
                // But in case it doesn't, we can dispatch the state change here if needed
              }
            }
          }}
          disabled={!isRelayRunning || loading}
          color="red"
          loading={loading}
        >
          Deactivate Relay
        </Button>

        <Button
          onClick={() => {
            console.log('🧪 Debug (Relay): Current Redux state:')
            console.log('  - Client logs:', global.logsClient?.length)
            console.log('  - Relay logs:', logsRelay?.length)
            console.log('  - Client active:', global.clientActive)
            console.log('  - Relay active:', relayActive)
            console.log('  - isRelayRunning (from useEltord):', isRelayRunning)
            dispatch(clearLogsRelay())
          }}
          color="orange"
          variant="light"
          size="sm"
        >
          Debug Clear Relay
        </Button>
        <Group ml="auto">
          <Text>Relay Status: {isRelayRunning ? 'Running' : 'Stopped'}</Text>
          <Circle color={isRelayRunning ? 'lightgreen' : '#FF6347'} />
        </Group>
      </Group>
      <Center>
        <Box
          className="log-window"
          style={{
            position: 'relative',
            padding: 4,
            borderRadius: 4,
            backgroundColor: '#1e1e1e',
            zIndex: 1,
          }}
        >
          <pre
            ref={preRef}
            style={{
              backgroundColor: '#1e1e1e',
              height: '250px',
              borderRadius: 4,
              fontFamily: 'monospace',
              color: '#d4d4d4',
              padding: 6,
              paddingTop: 0,
              overflow: 'auto',
              display: 'block',
              position: 'relative',
            }}
          >
            <LogViewer
              height="250px"
              className="mt-[-130px] z-10 relative max-w-full"
              mode="relay"
              scroll={false}
            />
          </pre>
          <Button
            size="xs"
            style={{ position: 'absolute', bottom: 4, right: 4, height: 24 }}
            onClick={() => dispatch(clearLogsRelay())}
          >
            Clear
          </Button>
        </Box>
      </Center>
      <Text>
        <b>2. OS Firewall</b> - Make sure to open the ORPort on your OS firewall
      </Text>
      {/* TODO read ports and IP from config */}
      <CopyableTextBox text="ufw allow 9996" />
      <Text>
        <b>3. Router Port Forward (NAT)</b> - Make sure to port forward the
        ORPort on your router if behind NAT. Or if your router supports UPnP,
        you can use
        <a href="https://miniupnp.tuxfamily.org/" target="_blank">
          {' '}
          miniupnp
        </a>
      </Text>
      {/* TODO read ports and IP from config */}
      <CopyableTextBox text="upnpc -a X.X.X.X 9996 9996 TCP" />
      <Text>
        <b>4. Get Paid</b> - Monitor your wallet for payments to your BOLT 12
        Offer
      </Text>
      <CopyableTextBox text={wallet.bolt12Offer} limitChars={80} />
      <Text>
        <b>5. Monitor</b> your relay with{' '}
        <a href="https://nyx.torproject.org/" target="_blank">
          {' '}
          nyx
        </a>
        : <br />
        (you might need to change the control port 8061 based on your torrc
        config)
      </Text>
      <CopyableTextBox text="nyx -i 127.0.0.1:8061" />
      <Text mb="xl"></Text>
    </Stack>
  )
}
