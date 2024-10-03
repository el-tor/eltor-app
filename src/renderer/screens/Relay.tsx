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
      <Text>Relay Active: {relayActive}</Text>
      <p>
        Run this command in the terminal to start the Tor Relay:
        <pre style={{backgroundColor: "white", color:"black", width: "980px"}}>
        /bin/bash -c "$(curl -fsSL https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/relay.sh)"
        </pre>
      </p>
    </Stack>
  );
};
