import React, { useEffect, useState } from "react";
import { Box, Group, Container } from "@mantine/core";
import { IconCircuitBattery, IconCoinBitcoin } from "@tabler/icons-react";
import { useLocation } from "wouter";
import eltorLogo from "./../assets/eltor-logo.png";
import classes from "../globals.module.css";

export default function Nav() {
  const [active, setActive] = useState("Connect");
  const [, navigate] = useLocation();
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    setIsLoaded(true);
  }, []);

  return (
    <Container
      w="768px"
      mt="sm"
      ml="xs"
      mr="xs"
      // bg="gray"
    >
      {isLoaded && (
        <Group align="center">
          <Box>
            <img
              src={eltorLogo}
              alt="Logo"
              height={50}
              style={{ cursor: "pointer" }}
              onClick={() => {
                navigate("/connect/false");
              }}
            />
          </Box>

          <Group>
            <a
              className={classes.link}
              data-active={
                window.location.pathname.includes("/connect") ||
                window.location.pathname.includes("/main_window") || undefined
              }
              href=""
              key={"Tor"}
              onClick={(event) => {
                event.preventDefault();
                setActive("Connect");
                try {
                  navigate("/connect/false");
                } catch (e) {}
              }}
            >
              <IconCircuitBattery className={classes.linkIcon} stroke={1.5} />
              <span>Connect to El Tor</span>
            </a>
            <a
              className={classes.link}
              data-active={window.location.pathname.includes("/relay") || undefined}
              key={"Host"}
              href=""
              onClick={(event) => {
                event.preventDefault();
                setActive("Relay");
                try {
                  navigate("/relay");
                } catch (e) {}
              }}
            >
              <IconCoinBitcoin className={classes.linkIcon} stroke={1.5} />
              <span>Host a Relay (get paid)</span>
            </a>
            <a
              className={classes.link}
              data-active={window.location.pathname.includes("/wallet") || undefined}
              href=""
              key={"wallet"}
              onClick={(event) => {
                event.preventDefault();
                setActive("Wallet");
                try {
                  navigate("/wallet");
                } catch (e) {}
              }}
            >
              <IconCoinBitcoin className={classes.linkIcon} stroke={1.5} />
              <span>Wallet</span>
            </a>
          </Group>
        </Group>
      )}
    </Container>
  );
}
