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
  Checkbox,
  Modal,
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
import {
  clearLogsRelay,
  setRelayEnabled,
  setClientEnabled,
} from '../globalStore'

export const Relay = () => {
  const { global, wallet } = useSelector((state) => state)
  const [loading, setLoading] = useState(false)
  const [modalOpened, setModalOpened] = useState(false)
  const {
    isRunning: isRelayRunning,
    loading: isRelayLoading,
    isLoadingDeactivate,
    isAnyModeRunning,
    activate,
    deactivate,
  } = useEltord({
    mode: 'relay',
  })
  const {
    logsRelay,
    relayActive,
    circuits,
    circuitInUse,
    relayEnabled,
    clientEnabled,
  } = useSelector((state) => state.global)
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
      <Group>
        <Title order={2}>Relay</Title>
        <Group ml="auto">
          <Text>Relay Status: {isRelayRunning ? 'Running' : 'Stopped'}</Text>
          <Circle color={isRelayRunning ? 'lightgreen' : '#FF6347'} />
        </Group>
      </Group>
      <Text>
        <b>1. Run a Relay</b> - and get paid for sharing your bandwidth
      </Text>

      <Checkbox
        label="Enable Relay"
        checked={relayEnabled}
        onChange={(event) => {
          if (event.currentTarget.checked) {
            dispatch(setRelayEnabled(true))
          } else {
            dispatch(setRelayEnabled(false))
          }
          setModalOpened(true)
        }}
      />
      <Checkbox
        label="Enable Client - You can disable this if you only want to run a relay and not use the El Tor network yourself"
        checked={clientEnabled}
        onChange={(event) => {
          if (event.currentTarget.checked) {
            dispatch(setClientEnabled(true))
          } else {
            dispatch(setClientEnabled(false))
          }
          setModalOpened(true)
        }}
      />

      <Text>
        <b>2. Get Paid</b> - You will get paid out to this wallet offer
      </Text>
      <CopyableTextBox text={wallet.bolt12Offer} limitChars={80} />
      <Text>
        <b>3. OS Firewall</b> - Make sure to open this onion router port on your
        OS firewall
      </Text>
      {/* TODO read ports and IP from config */}
      <CopyableTextBox text="ufw allow 9996" />
      <Text>
        <b>4. Router Port Forward (NAT)</b> - Make sure to port forward the
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
        <b>5. Monitor</b> your relay with{' '}
        <a href="https://nyx.torproject.org/" target="_blank">
          {' '}
          nyx
        </a>
        : <br />
        (you might need to change the control port 7781 based on your torrc
        config)
      </Text>
      <CopyableTextBox text="nyx -i 127.0.0.1:7781" />
      <Text mb="xl"></Text>
      <Modal
        opened={modalOpened}
        onClose={() => setModalOpened(false)}
        title="Action Required"
        centered
      >
        <Text mb="md">
          Make sure to restart the connection on the home screen for this
          setting to apply (*if it's currently running).
        </Text>
        <Group justify="flex-end">
          <Button onClick={() => setModalOpened(false)}>Ok</Button>
        </Group>
      </Modal>
    </Stack>
  )
}
