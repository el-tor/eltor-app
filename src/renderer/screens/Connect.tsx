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
import { setCommandOutput } from "renderer/globalStore";
import { useDispatch, useSelector } from "renderer/hooks";
import styles from "./../globals.module.css";

const { electronEvents } = window;

export const Connect = () => {
  const params: any = useParams();
  const [loading, setLoading] = useState(false);
  const dispatch = useDispatch();
  const { commandOutput, torActive } = useSelector((state) => state.global);
  const preRef = useRef<HTMLPreElement>(null);
  const [circuitsToPay, setCircuitsToPay] = useState<Array<string>>([]);

  useEffect(() => {
    // Listen for 'onTorStdout' event via the exposed electronAPI
    electronEvents.onTorStdout((event: any, data: any) => {
      dispatch(setCommandOutput(commandOutput + "\n\n" + data));
    });
    electronEvents.onPayCircuit((event, circuit) => {
      setCircuitsToPay(circuit);
    });
    electronEvents.onNavigateToDeactivateConnect(() => {
      dispatch(setCommandOutput("Deactivated"));
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
        <Title order={2}>{torActive ? "Connected" : "Not connected"}</Title>
        <Group ml="auto">
          <Center> {loading && <Loader size="sm" />}</Center>
        </Group>
      </Group>

      <Box
        style={{
          maxWidth: styles.maxWidth,
          position: "relative",
          padding: 6,
          borderRadius: 4,
        }}
        bg="white"
      >
        <pre
          ref={preRef}
          style={{
            backgroundColor: "white",
            height: "640px",
            borderRadius: 4,
            fontFamily: "monospace",
            color: "black",
            padding: 12,
            overflow: "auto",
            display: "block",
            position: "relative",
          }}
        >
          {commandOutput}
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
