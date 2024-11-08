import { Stack, Title, Text, Group, Center, Loader } from "@mantine/core";
import { useState } from "react";
import { Circle } from "renderer/components/Circle";
import CopyableTextBox from "renderer/components/CopyableTextBox";
import { useSelector } from "../hooks";

export const Relay = () => {
  const { global, wallet } = useSelector((state) => state);
  const [loading, setLoading] = useState(false);

  return (
    <Stack>
      <Title order={2}>Relay</Title>
      {/* <Group w="100%">
        <Circle color={relayActive ? "lightgreen" : "#FF6347"} />
        <Title order={2}>{relayActive ? "Relay Active" : "Relay not active"}</Title>
        <Group ml="auto">
          <Center> {loading && <Loader size="sm" />}</Center>
        </Group>
      </Group> */}
      <Text>
        <b>1. Run</b> this command in your terminal to start the El Tor Relay
      </Text>
      <CopyableTextBox text='/bin/bash -c "$(curl -fsSL https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/relay.sh)"' />
      <Text>
        <b>2. Monitor</b> your relay with{" "}
        <a href="https://nyx.torproject.org/" target="_blank">
          {" "}
          nyx
        </a>
        : <br />
        (you might need to change the control port 8061 based on your torrc
        config)
      </Text>
      <CopyableTextBox text="nyx -i 127.0.0.1:8061" />
      <Text>
        <b>3. Firewall</b> - Make sure to open the ORPort on your router
        <br />
        Or if your router supports UPnP, you can use
        <a href="https://github.com/Yawning/tor-fw-helper" target="_blank">
          {" "}
          tor-fw-helper
        </a>
      </Text>
      <CopyableTextBox text="./tor-fw-helper -p 5061:5061" />
      <Text>
        <b>4. Get Paid</b> - Monitor your wallet for payments to your BOLT 12 Offer
      </Text>
      <CopyableTextBox text={wallet.bolt12Offer} limitChars={80} />
    </Stack>
  );
};
