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
} from "@mantine/core";
import { useEffect, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { Circle } from "renderer/components/Circle";
import { setCommandOutput, setCircuits } from "renderer/globalStore";
import { useDispatch, useSelector } from "renderer/hooks";
import styles from "./../globals.module.css";
import MapComponent from "renderer/components/Map/MapComponent";
import "./Connect.css";

const { electronEvents } = window;

export const Connect = () => {
  const params: any = useParams();
  const [loading, setLoading] = useState(false);
  const dispatch = useDispatch();
  const { commandOutput, torActive, circuits } = useSelector(
    (state) => state.global
  );
  const preRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    // Listen for 'onTorStdout' event via the exposed electronAPI
    electronEvents.onTorStdout((event: any, data: any) => {
      dispatch(setCommandOutput(commandOutput + "\n\n" + data));
    });
    electronEvents.onPayCircuit((event, circuits) => {
      dispatch(
        setCommandOutput(
          commandOutput + "\n\nPay Circuits: " + JSON.stringify(circuits)
        )
      );
      dispatch(setCircuits(circuits));
    });
    electronEvents.onNavigateToDeactivateConnect(() => {
      dispatch(setCommandOutput("Deactivated"));
      dispatch(setCircuits([]));
    });
  }, []);

  useEffect(() => {
    if (preRef.current) {
      preRef.current.scrollTop = preRef.current.scrollHeight;
    }
  }, [commandOutput]);

  return (
    <Stack>
      <Group w="100%">
        <Circle color={torActive ? "lightgreen" : "#FF6347"} />
        <Title order={3}>{torActive ? "Connected" : "Not connected"}</Title>
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
        />
        <Group ml="auto">
          <Center> {loading && <Loader size="sm" />}</Center>
        </Group>
      </Group>
      <MapComponent circuits={circuits} h={500} />
      <Box
        style={{
          maxWidth: styles.maxWidth,
          position: "relative",
          padding: 4,
          borderRadius: 4,
          backgroundColor: "#1e1e1e",
          marginTop: -130,
          zIndex: 1,
        }}
      >
        <pre
          ref={preRef}
          style={{
            backgroundColor: "#1e1e1e",
            height: "220px",
            borderRadius: 4,
            fontFamily: "monospace",
            color: "#d4d4d4",
            padding: 6,
            paddingTop: 0,
            overflow: "auto",
            display: "block",
            position: "relative",
          }}
        >
          {commandOutput}
          <span className="blink-cursor">
            &nbsp;
          </span>
        </pre>
        <Button
          size="xs"
          style={{ position: "absolute", bottom: 4, right: 4, height: 24 }}
          onClick={() => dispatch(setCommandOutput(""))}
        >
          Clear
        </Button>
      </Box>
    </Stack>
  );
};
