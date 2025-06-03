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
  setCommandOutput,
  setCircuits,
  setCircuitInUse,
  Circuit,
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
  const { isRunning, activate, deactivate } = useEltord()
  const { commandOutput, torActive, circuits, circuitInUse } = useSelector(
    (state) => state.global,
  )
  const preRef = useRef<HTMLPreElement>(null)
  const [logs, setLogs] = useState<LogEntry[]>([])
  

  useEffect(() => {
    if (preRef.current) {
      preRef.current.scrollTop = preRef.current.scrollHeight
    }
  }, [commandOutput])

  return (
    <Stack>
      <Group w="100%">
        {/* {torActive ? (
          <Text>Click "Deactivate" in the menu to disconnect. <br/>To Connect to El Tor open a browser and configure it use a socks5 proxy 127.0.0.1:18058</Text>
        ) : (
          <Text>Click "Activate" in the OS tray menu to connect</Text>
        )} */}

        <Group mb="md">
          <Badge color={isTauri() ? 'blue' : 'green'}>
            {isTauri() ? '🖥️ Desktop Mode' : '🌐 Web Mode'}
          </Badge>
          <Text>Eltord Status: {isRunning ? '🟢 Running' : '🔴 Stopped'}</Text>
        </Group>

        <Group>
          <Button
            onClick={activate}
            disabled={isRunning || loading}
            color="green"
            loading={loading}
          >
            Activate
          </Button>

          <Button
            onClick={deactivate}
            disabled={!isRunning || loading}
            color="red"
            loading={loading}
          >
            Deactivate
          </Button>
          
          {isTauri() && (
            <Button
              onClick={async () => {
                try {
                  const result = await apiService.testLogEvent()
                  console.log('Test log event result:', result)
                } catch (error) {
                  console.error('Test log event failed:', error)
                }
              }}
              color="blue"
              variant="light"
              size="sm"
            >
              Test Event
            </Button>
          )}
        </Group>

        {/* <Title order={3}>{torActive ? "Connected" : "Not connected"}</Title>
        <Switch
          checked={torActive}
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
          <Circle color={torActive ? 'lightgreen' : '#FF6347'} />
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
            logs={logs}
            setLogs={setLogs}
          />
          <span className="blink-cursor">&nbsp;</span>
        </pre>
        <Button
          size="xs"
          style={{ position: 'absolute', bottom: 4, right: 4, height: 24 }}
          onClick={() => setLogs([])}
        >
          Clear
        </Button>
      </Box>
    </Stack>
  )
}
