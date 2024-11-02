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
        1. Run this command in the terminal to start the Tor Relay:
      </Text>
      <CopyableTextBox text='/bin/bash -c "$(curl -fsSL https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/relay.sh)"' />
      <Text>
        2. Monitor your relay: <br/>(you might need to change the control port 8061 based on your torrc config):
      </Text>
      <CopyableTextBox text="nyx -i 127.0.0.1:8061" />
      <Text>
        3. Firewall: Make sure to open the OrPort on your router. <br/>
        Or if your router supports UPnP, you can use the following tool to open the port 
        <a href="https://github.com/Yawning/tor-fw-helper" target="_blank">tor-fw-helper</a>:
      </Text>
      <CopyableTextBox text='./tor-fw-helper -p 5061:5061' />
    </Stack>
  );
};
