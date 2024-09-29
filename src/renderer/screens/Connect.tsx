import { Title, Stack, Text } from "@mantine/core";
import { useParams } from "react-router-dom";
import { useLocalStorage } from "usehooks-ts";

export const Connect = () => {
  const params: any = useParams();
  const [torActive, setTorActive, removeTorActive] = useLocalStorage("torActive", "false");
  return (
    <Stack>
      <Title order={2}>
        Connect
      </Title>
      <Title order={3}>{torActive === "true" ? "Connected" : "Not Connected"}</Title>
      <Text>Tor Active: {torActive}</Text>
    </Stack>
  );
};
