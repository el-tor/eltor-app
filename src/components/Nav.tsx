import React, { useState } from "react";
import { Box, ScrollArea, Center, Group } from "@mantine/core";
import { IconCircuitBattery, IconCoinBitcoin } from "@tabler/icons-react";
import { useLocation } from "wouter";
import eltorLogo from "./../assets/eltor-logo.png";
import classes from "../globals.module.css";

export default function Nav({
  children,
}: Readonly<{
  children?: React.ReactNode;
}>) {
  const [active, setActive] = useState("Tor");
  const [, navigate] = useLocation();

  return (
    <Box p="sm">
      <Group p="lg" justify="space-between">
        <Center>
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

          <a
            className={classes.link}
            data-active={window.location.pathname === "/connect" || undefined}
            href={"/connect"}
            key={"Tor"}
            onClick={(event) => {
              event.preventDefault();
              setActive("Tor");
              navigate("/connect");
            }}
          >
            <IconCircuitBattery className={classes.linkIcon} stroke={1.5} />
            <span>Connect to El Tor</span>
          </a>
          <a
            className={classes.link}
            data-active={window.location.pathname === "/host" || undefined}
            href={"/host"}
            key={"Host"}
            onClick={(event) => {
              event.preventDefault();
              setActive("Host");
              navigate("/host");
            }}
          >
            <IconCoinBitcoin className={classes.linkIcon} stroke={1.5} />
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
}
