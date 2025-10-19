import {
  Title,
  Stack,
  Text,
  Loader,
  Group,
  Collapse,
  Center,
  Button,
  Box,
  Badge,
  Notification,
  Progress,
  Tooltip,
} from '@mantine/core'
import { useEffect, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { Circle } from '../components/Circle'
import LogViewer from '../components/LogViewer'
import { clearLogsClient } from '../globalStore'
import { useDispatch, useSelector } from '../hooks'
// @ts-ignore
import styles from '../globals.module.css'
import MapComponent from '../components/Map/MapComponent'
import './Connect.css'
import { useEltord } from '../hooks/useEltord'
import { useBootstrapping } from '../hooks/useBootstrapping'
import { apiService } from '../services/apiService'
import { useDisclosure } from '@mantine/hooks'
import { IconChevronDown, IconPlug } from '@tabler/icons-react'
import CopyableTextBox from '../components/CopyableTextBox'
import { Wizard } from '../features/wizard/Wizard'
import { SocksProxyHelp } from '../components/SocksProxyHelp'

export const Connect = () => {
  const params: any = useParams()
  const [loading, setLoading] = useState(false)
  const [debugInfo, setDebugInfo] = useState<any>(null)
  const dispatch = useDispatch()
  const navigate = useNavigate()
  const [opened, { toggle }] = useDisclosure(false)
  const [showSocksModal, setShowSocksModal] = useState(false)
  const { defaultWallet } = useSelector((state) => state.wallet)
  const [openedWizard, { open: openWizard, close: closeWizard }] =
    useDisclosure(false)

  const {
    logsClient,
    logsRelay,
    clientActive,
    relayActive,
    circuits,
    circuitInUse,
    activeMode,
    relayEnabled,
    clientEnabled,
  } = useSelector((state) => state.global)
  const mode =
    relayEnabled && clientEnabled ? 'both' : relayEnabled ? 'relay' : 'client'
  const socksPort =
    mode === 'client'
      ? debugInfo?.torrc_socks_port
      : debugInfo?.torrc_relay_socks_port
  const {
    isRunning,
    loading: isLoadingActivate,
    isLoadingDeactivate,
    activate,
    deactivate,
  } = useEltord({
    mode,
  })

  // Use bootstrapping hook to monitor Tor connection progress
  const {
    progress: bootstrapProgress,
    isBootstrapping: showBootstrapping,
    reset: resetBootstrapping,
    start: startBootstrapping,
  } = useBootstrapping({
    logs: logsClient,
    onComplete: () => setShowSocksModal(true),
  })

  const preRef = useRef<HTMLPreElement>(null)
  // const [logs, setLogs] = useState<LogEntry[]>([])

  useEffect(() => {
    if (preRef.current) {
      preRef.current.scrollTop = preRef.current.scrollHeight
    }
  }, [logsClient])

  // Add debug effect to log frontend state
  useEffect(() => {
    console.log('🔍 Connect Page - Current Redux State:')
    console.log('  - Client logs count:', logsClient?.length)
    console.log('  - Relay logs count:', logsRelay?.length)
    console.log('  - Client active:', clientActive)
    console.log('  - Relay active:', relayActive)
    console.log('  - Active mode:', activeMode)
  }, [logsClient, logsRelay, clientActive, relayActive, activeMode])

  // Fetch debug info on component mount
  useEffect(() => {
    const fetchDebugInfo = async () => {
      try {
        const info = await apiService.getDebugInfo()
        setDebugInfo(info)
        console.log('📋 Debug info loaded:', info)
      } catch (error) {
        console.error('❌ Failed to fetch debug info:', error)
      }
    }

    fetchDebugInfo()
  }, [])

  return (
    <Stack>
      <Group w="100%">
        {defaultWallet === 'none' && <Wizard close={closeWizard} />}

        <Group>
          {/* {isTauri() && (
            <Button
              onClick={async () => {
                try {
                  console.log('🧪 Testing Tauri log event...')
                  const result = await apiService.testLogEvent()
                  console.log('✅ Test log event result:', result)
                } catch (error) {
                  console.error('❌ Test log event failed:', error)
                }
              }}
              color="blue"
              variant="light"
              size="sm"
            >
              Test Tauri Event
            </Button>
          )} */}

          {/* <Button
            onClick={() => {
              console.log('🧪 Debug: Current Redux state:')
              console.log('  - Client logs:', logsClient?.length)
              console.log('  - Relay logs:', logsRelay?.length)
              console.log('  - Client active:', clientActive)
              console.log('  - Relay active:', relayActive)
              dispatch(clearAllLogs())
            }}
            color="orange"
            variant="light"
            size="sm"
          >
            Debug Clear All
          </Button> */}
        </Group>

        {/* <Title order={3}>{clientActive ? "Connected" : "Not connected"}</Title>
        <Switch
          checked={clientActive}
          onChange={(checked) => {
            setLoading(true);
            if (checked) {
            
            } else {
             
            }
            setLoading(false);
          }}
          color="purple"
        /> */}
        <Group ml="auto">
          <Center> {loading && <Loader size="sm" />}</Center>
        </Group>
      </Group>
      
      <Box className="map-container-mobile" style={{ position: 'relative', marginTop: '-20px' }}>
        <MapComponent h={500} />
        
        {/* Activate/Deactivate buttons - top left */}
        <Box
          className="glass-effect map-overlay-top-left"
          style={{
            padding: '12px',
          }}
        >
          <Group gap="xs">
            <Button
              onClick={async () => {
                // Reset any previous state
                resetBootstrapping()
                setShowSocksModal(false)
                
                // Start bootstrapping UI immediately at 1%
                startBootstrapping()
                
                await activate()
                
                // Don't show SOCKS modal immediately - wait for bootstrapping to complete
                // The useBootstrapping hook will show it after 100%
              }}
              disabled={isRunning || loading}
              color="green"
              loading={loading || isLoadingActivate}
            >
              {isRunning ? 'Active' : 'Activate'}
            </Button>

            <Button
              onClick={async () => {
                try {
                  await deactivate()
                  setShowSocksModal(false)
                  resetBootstrapping()
                } catch (error) {
                  console.error('❌ [Connect] Deactivate error:', error)
                  // Handle the case where backend says "No eltord client process is currently running"
                  // This means the frontend state is out of sync with backend
                  if (
                    error instanceof Error &&
                    error.message.includes(
                      'No eltord client process is currently running',
                    )
                  ) {
                    console.log(
                      '🔄 [Connect] Backend says client not running, syncing frontend state',
                    )
                    // The useEltord hook should handle state updates through the 'eltord-error' event
                    // But in case it doesn't, we can dispatch the state change here if needed
                  }
                }
              }}
              disabled={!isRunning || loading}
              color="red"
              loading={loading || isLoadingDeactivate}
            >
              Deactivate
            </Button>
          </Group>
        </Box>
        
        {/* Status overlay - top right */}
        <Box
          className="glass-effect map-overlay-top-right"
          style={{
            padding: '12px 16px',
            maxWidth: '200px',
          }}
        >
          <Stack align="left" gap="8px">
            <Badge
              style={{ cursor: 'pointer' }}
              onClick={() => navigate('/relay')}
            >
              Mode: {mode === 'both' ? 'Client+Relay' : mode}
            </Badge>
            <Group gap="xs">
              <Text>Client</Text>
              <Circle
                color={isRunning && clientEnabled ? 'lightgreen' : '#FF6347'}
              />
            </Group>
            <Group gap="xs">
              <Text>Relay&nbsp;</Text>
              <Circle
                color={isRunning && relayEnabled ? 'lightgreen' : '#FF6347'}
              />
            </Group>
            <Group gap="xs">
              <Text>{defaultWallet !== 'none' ? defaultWallet : ''}</Text>
            </Group>
            {circuitInUse.id && isRunning && (() => {
              const circuitIdStr = String(circuitInUse.id)
              return (
                <Tooltip label={circuitIdStr} disabled={circuitIdStr.length <= 6}>
                  <Text style={{ cursor: circuitIdStr.length > 6 ? 'help' : 'default' }}>
                    Circuit: {circuitIdStr.length > 6 ? `${circuitIdStr.substring(0, 6)}...` : circuitIdStr}
                  </Text>
                </Tooltip>
              )
            })()}
          </Stack>
        </Box>
        
        {(showBootstrapping || showSocksModal) && (
          <Center className="bootstrap-notification">
            <Box 
              className="glass-effect"
              style={{ 
                pointerEvents: 'auto',
              }}
            >
              <Notification
                w="500px"
                title={
                  showBootstrapping ? (
                    'Connecting to the El Tor Network...'
                  ) : (
                    <Group gap="xs" wrap="nowrap">
                      <Text>Connected! Socks5 Proxy Ready</Text>
                      <SocksProxyHelp
                        hostname={window.location.hostname}
                        port={socksPort}
                      />
                    </Group>
                  )
                }
                icon={<IconPlug />}
                onClose={() => {
                  setShowSocksModal(false)
                  resetBootstrapping()
                }}
                color={showBootstrapping ? 'blue' : 'green'}
                styles={{
                  root: {
                    backgroundColor: 'transparent',
                    border: 'none',
                  }
                }}
              >
                {showBootstrapping ? (
                  <Stack gap="md">
                    <Text size="sm" c="dimmed">
                      Bootstrapping {bootstrapProgress}%
                    </Text>
                    <Progress
                      value={bootstrapProgress}
                      size="lg"
                      radius="xl"
                      animated
                      striped
                      color={bootstrapProgress === 100 ? 'green' : 'blue'}
                    />
                    {bootstrapProgress === 100 && (
                      <Text size="xs" c="green" ta="center" fw={600}>
                        ✅ Connection established!
                      </Text>
                    )}
                  </Stack>
                ) : (
                  <>
                    <Text mb="xs" mt="xs" size="sm">
                      Open a browser (System-Wide Proxy) and configure it to use a Socks5 proxy at:
                    </Text>
                    <CopyableTextBox
                      text={`${window.location.hostname}:${socksPort}`}
                      h="44px"
                    />
                  </>
                )}
              </Notification>
            </Box>
          </Center>
        )}
      </Box>
      <Center>
        <Box
          className="terminal-container"
          style={{
            width: '100%',
            position: 'relative',
            padding: 4,
            borderRadius: 4,
            backgroundColor: '#1e1e1e',
            marginTop: -130,
            zIndex: 1,
          }}
        >
          <pre
            ref={preRef}
            style={{
              backgroundColor: '#1e1e1e',
              height: '260px',
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
              height="260px"
              className="mt-[-130px] z-10 relative max-w-full"
              mode="client"
              scroll={false}
            />
          </pre>
          <Button
            size="xs"
            style={{ position: 'absolute', bottom: 4, right: 4, height: 24 }}
            onClick={() => dispatch(clearLogsClient())}
          >
            Clear
          </Button>
        </Box>
      </Center>
      <Box mb="xl">
        <Group mb={5} mt="xs">
          <Button onClick={toggle} rightSection={<IconChevronDown size={14} />}>
            Show Debug Info
          </Button>
          {debugInfo?.torrc_path && (
            <Text size="lg" c="dimmed">
              Edit your config at the torrc path:{' '}
              {mode === 'client'
                ? debugInfo.torrc_path
                : `${debugInfo.torrc_path}.relay`}
            </Text>
          )}
        </Group>

        <Collapse in={opened}>
          <Box
            mt="lg"
            style={{
              backgroundColor: '#1e1e1e',
              borderRadius: 4,
              fontFamily: 'monospace',
              color: '#d4d4d4',
              padding: 6,
              paddingTop: 0,
              display: 'block',
              position: 'relative',
              overflowX: 'auto',
              overflowY: 'hidden',
              whiteSpace: 'nowrap',
            }}
          >
            <Title order={5}>Settings</Title>
            <Title order={5}>=======</Title>
            <Text>
              <pre
                style={{
                  padding: '16px',
                  borderRadius: '4px',
                  overflow: 'auto',
                  fontSize: '14px',
                  fontFamily: 'monospace',
                }}
              >
                {JSON.stringify(
                  { ...debugInfo, torrc_file: 'see below', torrc_relay_file: 'see below' },
                  null,
                  2,
                )}
              </pre>
            </Text>
            <Title order={5}>Raw torrc File (Client)</Title>
            <Title order={5}>========================</Title>
            <pre>{debugInfo?.torrc_file ?? ''}</pre>
            
            <Title order={5} mt="lg">Raw torrc Relay File</Title>
            <Title order={5}>====================</Title>
            <pre>{debugInfo?.torrc_relay_file ?? ''}</pre>
          </Box>
        </Collapse>
      </Box>
    </Stack>
  )
}
