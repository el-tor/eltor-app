import { Title, Stack, Text, Loader } from "@mantine/core";
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { useLocalStorage, useTimeout } from "usehooks-ts";

const { electronEvents } = window;

export const Connect = () => {
  const params: any = useParams();
  const [torActive, setTorActive, removeTorActive] = useLocalStorage(
    "torActive",
    "false"
  );
  const [loading, setLoading] = useState(false);
  const [commandOutput, setCommandOutput] = useState("");
  const [circuitsToPay, setCircuitsToPay] = useState<Array<string>>([]);

  useEffect(() => {
    // Listen for 'onTorStdout' event via the exposed electronAPI
    electronEvents.onTorStdout((event: any, data: any) => {
      setCommandOutput(data);
    });
    electronEvents.onPayCircuit((event, circuit) => {
      setCircuitsToPay(circuit);
    });
    electronEvents.onNavigateToDeactivateConnect(() => {
      setCommandOutput("");
    });
  }, []);

  return (
    <Stack>
      <Title order={3}>
        {torActive === "true" ? "Connected" : "Not Connected"}
      </Title>
      <Text
        color={torActive === "false" ? "red" : "green"}
        style={{ fontSize: "20px" }}
      >
        Tor Active: {torActive}
      </Text>
      {loading && <Loader />}

      {circuitsToPay && (
        <Text style={{ fontSize: "20px" }}>
          {JSON.stringify(circuitsToPay)}
        </Text>
      )}
      <pre
        style={{
          backgroundColor: "white",
          width: "860px",
          height: "600px",
          borderRadius: 4,
          fontFamily: "monospace",
          color: "black",
          padding: 12,
          overflow: "auto",
          display: commandOutput === "" ? "none" : "block",
        }}
      >
        {commandOutput}
      </pre>
    </Stack>
  );
};
