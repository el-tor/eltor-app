import { Title, Stack, Text } from "@mantine/core";
import eltorLogo from "../assets/eltor-logo.png";
import { useParams } from "wouter";
import { useLocalStorage } from "usehooks-ts";

export const Connect = () => {
  const params: any = useParams();
  const [torActive, setTorActive, removeTorActive] = useLocalStorage("torActive", "false");

  console.log("torActive", torActive);

  console.log(params);
  const isConnected = params?.connected ? params.connected === "true" : false;
  console.log(params);
  return (
    <Stack>
      <Title order={2}>
        Connect
      </Title>
      <Title order={3}>{isConnected ? "Connected" : "Not Connected"}</Title>

      <Text>Tor Active: {torActive}</Text>
      {/* <img src={eltorLogo} alt="El Tor" width={250} /> */}
    </Stack>
  );
};
