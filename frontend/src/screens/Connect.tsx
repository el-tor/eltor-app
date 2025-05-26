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
import {
  setCommandOutput,
  setCircuits,
  setCircuitInUse,
  Circuit,
} from '../globalStore'
import { useDispatch, useSelector } from '../hooks'
import styles from '../globals.module.css'
import MapComponent from '../components/Map/MapComponent'
import './Connect.css'
import { useEltord } from '../hooks/useEltord'
import { isTauri } from '../utils/platform'


const { electronEvents } = window

export const Connect = () => {
  const params: any = useParams()
  const [loading, setLoading] = useState(false)
  const dispatch = useDispatch()
  const { isRunning, activate, deactivate } = useEltord()
  const { commandOutput, torActive, circuits, circuitInUse } = useSelector(
    (state) => state.global,
  )
  const preRef = useRef<HTMLPreElement>(null)

  useEffect(() => {
    if (!electronEvents?.onTorStdout) return
    // Listen for 'onTorStdout' event via the exposed electronAPI
    electronEvents.onTorStdout((event: any, data: any) => {
      dispatch(setCommandOutput(commandOutput + '\n\n' + data))
    })
    electronEvents.onPayCircuit((event, circuitResp) => {
      dispatch(
        setCommandOutput(
          commandOutput +
            `
            \n\nPay Circuits:  ${JSON.stringify(circuitResp.circuits)}
          `,
        ),
      )
      dispatch(setCircuits(circuitResp.circuits))
      if (circuitInUse.id !== circuitResp.circuitInUse.id) {
        dispatch(setCircuitInUse(circuitResp.circuitInUse))
      }
    })
    // electronEvents.onCircuitRenew((event, circuitRenew) => {
    //   if (circuitRenew.circuit.relays.length >= 3) {
    //     dispatch(
    //       setCommandOutput(
    //         commandOutput +
    //           `
    //         \n\nRenewed Circuits:  ${JSON.stringify(circuitRenew.circuit)}
    //         \nHop 1: ${circuitRenew.circuit.relays[0]?.nickname} - ${
    //             circuitRenew.circuit.relays[0]?.ip
    //           },
    //         \nHop 2: ${circuitRenew.circuit.relays[1]?.nickname} - ${
    //             circuitRenew.circuit.relays[1]?.ip
    //           },
    //         \nHop 3: ${circuitRenew.circuit.relays[2]?.nickname} - ${
    //             circuitRenew.circuit.relays[2]?.ip
    //           },
    //       `
    //       )
    //     );
    //     dispatch(setCircuitInUse(circuitRenew.circuit));
    //   }
    // });
    electronEvents.onNavigateToDeactivateConnect(() => {
      dispatch(setCommandOutput('Deactivated'))
      dispatch(setCircuits([]))
      dispatch(setCircuitInUse({} as Circuit))
    })
  }, [])

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
        </Group>

        {/* <Title order={3}>{torActive ? "Connected" : "Not connected"}</Title>
        <Switch
          checked={torActive}
          onChange={(checked) => {
            setLoading(true);
            if (checked) {
            electronEvents.menuActivateConnect(()=>{});
            } else {
              electronEvents.menuDeactivateConnect(()=>{});
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
          {commandOutput}
          <span className="blink-cursor">&nbsp;</span>
        </pre>
        <Button
          size="xs"
          style={{ position: 'absolute', bottom: 4, right: 4, height: 24 }}
          onClick={() => dispatch(setCommandOutput(''))}
        >
          Clear
        </Button>
      </Box>
    </Stack>
  )
}
