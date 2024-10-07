import { Stack, Title, Text } from "@mantine/core";
import { useLocalStorage } from "usehooks-ts";

export const Relay = () => {
  const [relayActive, setRelayActive, removeRelayActive] = useLocalStorage(
    "relayActive",
    "false"
  );

  return (
    <Stack>
      <Title order={2}>Relay</Title>
      <Text
        color={relayActive === "false" ? "red" : "green"}
        style={{ fontSize: "20px" }}
      >
        Relay Active: {relayActive}
      </Text>
      <Text>
        Run this command in the terminal to start the Tor Relay:
        <pre
          style={{
            backgroundColor: "white",
            color: "black",
            width: "980px",
            padding: 6,
          }}
        >
          /bin/bash -c "$(curl -fsSL
          https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/relay.sh)"
        </pre>
      </Text>
      <Text>
        Run this command to monitor the relay:
        <pre
          style={{
            backgroundColor: "white",
            color: "black",
            width: "230px",
            padding: 6,
          }}
        >
          nyx -i 127.0.0.1:8061
        </pre>
      </Text>
    </Stack>
  );
};
