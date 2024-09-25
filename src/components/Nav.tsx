import React, { useState } from "react";
import { Box, ScrollArea, Center, Group } from "@mantine/core";
import { IconCircuitBattery, IconCoinBitcoin } from "@tabler/icons-react";
import { useLocation } from "wouter";
import eltorLogo from "./../assets/eltor-logo.png";

export default function Nav({
  children,
}: Readonly<{
  children?: React.ReactNode;
}>) {
  const [active, setActive] = useState("Tor");
  const [, navigate] = useLocation();

  return (
    <Box p="sm" bg="rgb(26, 26, 26)">
      <Group p="lg" justify="space-between">
        <Group>
          <Box>
            <img
              src={eltorLogo}
              alt="Logo"
              height={50}
              style={{ cursor: "pointer" }}
              onClick={() => {
                navigate("/");
              }}
            />
          </Box>
        </Group>
        <Center>
          <a
            style={{ color: "white" }}
            data-active={window.location.pathname === "/connect" || undefined}
            href={"/connect"}
            key={"Tor"}
            onClick={(event) => {
              event.preventDefault();
              setActive("Tor");
              navigate("/connect");
            }}
          >
            <IconCircuitBattery className="linkIcon" stroke={1.5} />
            <span>Connect to El Tor</span>
          </a>
          <a
            className="link"
            data-active={window.location.pathname === "/host" || undefined}
            href={"/host"}
            key={"Host"}
            onClick={(event) => {
              event.preventDefault();
              setActive("Host");
              navigate("/host");
            }}
          >
            <IconCoinBitcoin className="linkIcon" stroke={1.5} />
            <span>Host a Relay (and get paid)</span>
          </a>
        </Center>
      </Group>
      <ScrollArea
        style={{
          flexGrow: 1,
          backgroundColor: "#1a1a1a",
        }}
      >
        {children && children}
      </ScrollArea>
    </Box>
  );

  // return (
  //   <Box p="sm" bg="rgb(26, 26, 26)">
  //     <Box>
  //       <img
  //         src={eltorLogo}
  //         alt="Logo"
  //         height={50}
  //         style={{ cursor: "pointer" }}
  //         onClick={() => {
  //           navigate("/");
  //         }}
  //       />
  //     </Box>

  //     <Center>
  //       <a
  //         className="link"
  //         style={{ color: "white" }}
  //         data-active={window.location.pathname === "/connect" || undefined}
  //         href={"/connect"}
  //         key={"Tor"}
  //         onClick={(event) => {
  //           event.preventDefault();
  //           setActive("Tor");
  //           navigate("/connect");
  //         }}
  //       >
  //         <IconCircuitBattery className="linkIcon" stroke={1.5} />
  //         <span>Connect to El Tor</span>
  //       </a>
  //       <a
  //         className="link"
  //         style={{ color: "white" }}
  //         data-active={window.location.pathname === "/host" || undefined}
  //         href={"/host"}
  //         key={"Host"}
  //         onClick={(event) => {
  //           event.preventDefault();
  //           setActive("Host");
  //           navigate("/host");
  //         }}
  //       >
  //         <IconCoinBitcoin className="linkIcon" stroke={1.5} />
  //         <span>Host a Relay (and get paid)</span>
  //       </a>
  //     </Center>

  //     <ScrollArea
  //       style={{
  //         flexGrow: 1,
  //         backgroundColor: "#1a1a1a",
  //       }}
  //     >
  //       {children && children}
  //     </ScrollArea>
  //   </Box>
  // );
}
