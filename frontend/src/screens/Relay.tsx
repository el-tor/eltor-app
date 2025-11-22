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
  NumberInput,
} from '@mantine/core'
import { useEffect, useRef, useState } from 'react'
import { Circle } from '../components/Circle'
import CopyableTextBox from '../components/CopyableTextBox'
import { useDispatch, useSelector } from '../hooks'
import { isTauri } from '../utils/platform'
import { useEltord } from '../hooks/useEltord'
import LogViewer from '../components/LogViewer'
// @ts-ignore
import styles from '../globals.module.css'
import {
  clearLogsRelay,
  setRelayEnabled,
  setClientEnabled,
} from '../globalStore'
import { apiService } from '../services/apiService'

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
  const [localIp, setLocalIp] = useState<string>('X.X.X.X')
  const [rate, setRate] = useState<string | number>(1)
  const [orPort, setOrPort] = useState<number>(9996)
  const [controlPort, setControlPort] = useState<number>(7781)
  const [publicIp, setPublicIp] = useState<string>('X.X.X.X')

  const preRef = useRef<HTMLPreElement>(null)

  const handleRateChange = async (value: string | number) => {
    setRate(value)
    try {
      const numValue = typeof value === 'string' ? parseFloat(value) : value
      if (!isNaN(numValue) && numValue >= 0) {
        await apiService.updateRelayPaymentRate(numValue)
        console.log(`Payment rate updated to ${numValue} sat(s)/min`)
      }
    } catch (error) {
      console.error('Failed to update payment rate:', error)
    }
  }
  // const [logs, setLogs] = useState<LogEntry[]>([])

  useEffect(() => {
    if (preRef.current) {
      preRef.current.scrollTop = preRef.current.scrollHeight
    }
  }, [logsRelay])

  // Fetch local IP and payment rate from debug endpoint
  useEffect(() => {
    const fetchDebugInfo = async () => {
      try {
        const debugInfo = await apiService.getDebugInfo()
        if (debugInfo.local_ip) {
          setLocalIp(debugInfo.local_ip)
        } else {
          setLocalIp('X.X.X.X')
        }
        // Set initial payment rate from backend (convert msats to sats)
        if (
          debugInfo.payment_rate_msats !== undefined &&
          debugInfo.payment_rate_msats !== null
        ) {
          setRate(debugInfo.payment_rate_msats / 1000)
        }
        if (debugInfo.torrc_relay_or_port) {
          setOrPort(debugInfo.torrc_relay_or_port)
        }
        if (debugInfo.torrc_relay_control_port) {
          setControlPort(debugInfo.torrc_relay_control_port)
        }
        if (debugInfo.public_ip) {
          setPublicIp(debugInfo.public_ip)
        }
      } catch (error) {
        console.error('Failed to fetch debug info:', error)
        setLocalIp('X.X.X.X')
      }
    }
    fetchDebugInfo()
  }, [])

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
      <Group ml="auto" mt="-20">
        <Text color="dimmed" size="xs" mr="28">
          Activate on Home page
        </Text>
      </Group>
      <Text size="lg">Instructions on how to run a relay:</Text>
      <Group>
        <Text size="lg">
          <b>1. Run a Relay</b>
        </Text>
        <Text color="dimmed" size="md">
          and get paid for sharing your bandwidth
        </Text>
      </Group>
      <Group ml="32">
        <Checkbox
          size="md"
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
        <Group>
          <Checkbox
            size="md"
            label="Enable Client"
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
          <Text color="dimmed">
            - You can disable this if you only want to run a relay and not use
            the El Tor network yourself
          </Text>
        </Group>
      </Group>

      <Group mt="md">
        <Text size="lg">
          <b>2. Get Paid</b>
        </Text>
        <Text color="dimmed" size="md">
          Make sure to set your rate in sats per minute. You will get paid out
          to this BOLT12 offer.
        </Text>
      </Group>
      <Box ml="32">
        <Group mb="md">
          <NumberInput w="120" value={rate} onChange={handleRateChange} />
          <Text>sat(s) / min</Text>
        </Group>
        <CopyableTextBox
          text={wallet.bolt12Offer || 'Loading relay BOLT12 offer...'}
          limitChars={80}
        />
      </Box>

      <Group mt="md">
        <Text size="lg">
          <b>3. OS Firewall</b>
        </Text>
        <Text color="dimmed" size="md">
          Make sure to open the onion router port on your OS firewall (OrPort in
          torrc config) Detected public IP: {publicIp}
        </Text>
      </Group>
      <Box ml="32">
        <CopyableTextBox text={`ufw allow ${orPort}`} />
      </Box>

      <Group mt="md">
        <Text size="lg">
          <b>4. Router Port Forward (NAT)</b>
        </Text>
      </Group>

      <Box ml="32">
        <Text color="dimmed" size="md" mb="md">
          Make sure to port forward the OrPort on your router if behind NAT. See
          your router's documentation for specific instructions on how to set up
          port forwarding or visit{' '}
          <a
            href="https://www.wikihow.com/Set-Up-Port-Forwarding-on-a-Router"
            target="_blank"
          >
            https://www.wikihow.com/Set-Up-Port-Forwarding-on-a-Router
          </a>{' '}
          Or if your router supports UPnP, you can use
          <a href="https://miniupnp.tuxfamily.org/" target="_blank">
            {' '}
            miniupnp
          </a>{' '}
          with your local LAN IP address.
        </Text>
        <CopyableTextBox text={`upnpc -a ${localIp} ${orPort} ${orPort} TCP`} />
      </Box>

      <Group mt="md">
        <Text size="lg">
          <b>5. Monitor (Optional)</b>
        </Text>
      </Group>
      <Box ml="32">
        <Text mb="md" color="dimmed" size="md">
          Monitor your relay with{' '}
          <a href="https://nyx.torproject.org/" target="_blank">
            {' '}
            nyx
          </a>
          . Use it to help troubleshoot your Relay, check bandwidth usage, run
          commands, and view circuits. To login, use the command below with the
          default password `password1234_` or look in your `torrc.relay` file or
          env var `APP_ELTOR_TOR_RELAY_CONTROL_PASSWORD`
        </Text>
        <CopyableTextBox text={`nyx -i 127.0.0.1:${controlPort}`} />
      </Box>
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
