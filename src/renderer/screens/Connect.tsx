import { Title, Stack, Text, Loader } from "@mantine/core";
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { useLocalStorage, useTimeout } from "usehooks-ts";

const { api } = window;

export const Connect = () => {
  const params: any = useParams();
  const [torActive, setTorActive, removeTorActive] = useLocalStorage(
    "torActive",
    "false"
  );
  const [loading, setLoading] = useState(false);

  // const [commandOutput, setCommandOutput] = useState("");
  // const handleRunCommand = async () => {
  //   try {
  //     const output = await api.runCommand();
  //     setCommandOutput(output);
  //   } catch (error: any) {
  //     setCommandOutput(`Error: ${error?.message}`);
  //   }
  // };


  return (
    <Stack>
      <Title order={2}>Connect</Title>
      <Title order={3}>
        {torActive === "true" ? "Connected" : "Not Connected"}
      </Title>
      <Text>Tor Active: {torActive}</Text>
      {loading && <Loader />}

      {/* <pre style={{backgroundColor: "white", width: "100%", height: "400px", borderRadius: 4, fontFamily:"monospace", color:"black"}}>{commandOutput}</pre> */}
    </Stack>
  );
};
