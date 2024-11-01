import { Stack, Title, Text } from "@mantine/core";
import CopyableTextBox from "renderer/components/CopyableTextBox";
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
        color={relayActive === "false" ? "#FF6347" : "green"}
        style={{ fontSize: "20px" }}
      >
        Relay Active: {relayActive}
      </Text>
      <Text>
        Run this command in the terminal to start the Tor Relay:

        <CopyableTextBox text='/bin/bash -c "$(curl -fsSL https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/relay.sh)"' />

      </Text>
      <Text>
        Run this command to monitor the relay:
        <CopyableTextBox text="nyx -i 127.0.0.1:8061" />
      </Text>
    </Stack>
  );
};
