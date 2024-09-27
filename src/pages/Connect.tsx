import { Title, Stack } from "@mantine/core";
import eltorLogo from "../assets/eltor-logo.png";
import { useParams } from "wouter";

export const Connect = () => {
  const params: any = useParams();
  console.log(params);
  const isConnected = params?.connected ? params.connected === "true" : false;
  console.log(params);
  return (
    <Stack>
      <Title order={2} style={{ color: "white" }}>
        Home
      </Title>
      <Title order={3}>{isConnected ? "Connected" : "Not Connected"}</Title>
      {/* <img src={eltorLogo} alt="El Tor" width={250} /> */}
    </Stack>
  );
};
