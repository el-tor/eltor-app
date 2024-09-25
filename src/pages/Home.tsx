import { Center } from "@mantine/core";
import eltorLogo from "../assets/eltor-logo.png";

export const Home = () => {
  return (
    <Center>
      <h2 style={{ color: "white" }}>El Tor</h2>
      <img src={eltorLogo} alt="El Tor" width={250} />
    </Center>
  );
};
