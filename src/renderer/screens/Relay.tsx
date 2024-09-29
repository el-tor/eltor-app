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
    </Stack>
  );
};
