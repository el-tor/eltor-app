import {
  Title,
  Stack,
  Text,
  Loader,
  Group,
  Switch,
  Center,
  Button,
  Box,
  Badge,
} from '@mantine/core'
import { useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router-dom'
import { Circle } from '../components/Circle'
import LogViewer from '../components/LogViewer'
import {
  setLogsClient,
  setCircuits,
  setCircuitInUse,
  Circuit,
  clearLogsClient,
  clearAllLogs,
} from '../globalStore'
import { useDispatch, useSelector } from '../hooks'
// @ts-ignore
import styles from '../globals.module.css'
import MapComponent from '../components/Map/MapComponent'
import './Connect.css'
import { useEltord } from '../hooks/useEltord'
import { isTauri } from '../utils/platform'
import { LogEntry, apiService } from '../services/apiService'


export const Connect = () => {
  const params: any = useParams()
  const [loading, setLoading] = useState(false)
  const dispatch = useDispatch()
  const { isRunning, isAnyModeRunning, activate, deactivate } = useEltord()
  const { logsClient, logsRelay, clientActive, relayActive, circuits, circuitInUse, activeMode } = useSelector(
    (state) => state.global,
  )
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

  return (
    <Stack>
      <Group w="100%">
        {/* {torActive ? (
          <Text>Click "Deactivate" in the menu to disconnect. <br/>To Connect to El Tor open a browser and configure it use a socks5 proxy 127.0.0.1:18058</Text>
        ) : (
          <Text>Click "Activate" in the OS tray menu to connect</Text>
        )} */}

        {/* <Group mb="md">
          <Badge color={isTauri() ? 'blue' : 'green'}>
            {isTauri() ? '🖥️ Desktop Mode' : '🌐 Web Mode'}
          </Badge>
        </Group> */}

        <Group>
          <Button
            onClick={activate}
            disabled={isRunning || loading}
            color="green"
            loading={loading}
          >
            {isRunning ? 'Client Active' : 'Activate Client'}
          </Button>

          <Button
            onClick={async () => {
              try {
                await deactivate()
              } catch (error) {
                console.error('❌ [Connect] Deactivate error:', error)
                // Handle the case where backend says "No eltord client process is currently running"
                // This means the frontend state is out of sync with backend
                if (error instanceof Error && error.message.includes('No eltord client process is currently running')) {
                  console.log('🔄 [Connect] Backend says client not running, syncing frontend state')
                  // The useEltord hook should handle state updates through the 'eltord-error' event
                  // But in case it doesn't, we can dispatch the state change here if needed
                }
              }
            }}
            disabled={!isRunning || loading}
            color="red"
            loading={loading}
          >
            Deactivate Client
          </Button>
         
          
          {isTauri() && (
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
          )}
          
          <Button
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
          </Button>
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
          {circuitInUse.id && isRunning && <Text>Circuit: {circuitInUse.id}</Text>}
           <Text>
            Client Status: {isRunning ? 'Running' : 'Stopped'}
          </Text>
          <Circle color={isRunning ? 'lightgreen' : '#FF6347'} />
        </Group>
      </Group>
      <MapComponent h={500} />
      <Box
        style={{
          maxWidth: styles.maxWidth,
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
            height: '220px',
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
    </Stack>
  )
}
