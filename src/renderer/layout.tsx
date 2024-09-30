import React, { useEffect, useState } from "react";
import { Box, Group, Container } from "@mantine/core";
import { IconCircuitBattery, IconCoinBitcoin } from "@tabler/icons-react";
import eltorLogo from "./assets/eltor-logo.png";
import classes from "./globals.module.css";
import { NavLink, Outlet, useNavigate } from 'react-router-dom'
import { useLocalStorage } from "usehooks-ts";
const { api } = window

export function Layout() {
  const [active, setActive] = useState("Connect");
  const navigate = useNavigate();
  const [isLoaded, setIsLoaded] = useState(false);
  const [torActive, setTorActive, removeTorActive] = useLocalStorage(
    "torActive",
    "false"
  );
  const [relayActive, setRelayActive, removeRelayActive] = useLocalStorage(
    "relayActive",
    "false"
  );

  useEffect(() => {
    setIsLoaded(true);
  }, []);

  
  useEffect(() => {
    // Connect
    api.onNavigateToConnect(() => {
      navigate("/connect");
      api.menuActivateConnect(()=>{});
      setTorActive("true");
    });
    api.onNavigateToDeactivateConnect(() => {
      navigate("/connect");
      api.menuDeactivateConnect(()=>{});
      setTorActive("false");
    });

    // Relay
    api.onNavigateToRelay(() => {
      navigate("/relay");
      api.menuActivateRelay(()=>{});
      setRelayActive("true");
    });
    api.onNavigateToDeactivateRelay(() => {
      navigate("/relay");
      api.menuDeactivateRelay(()=>{});
      setRelayActive("false");
    });

    // Wallet
    api.onNavigateToWallet(() => {
      navigate("/wallet");
    });
  }, []);

  return (
    <Container
      w="768px"
      mt="sm"
      ml="xs"
      mr="xs"
      maw="768px"
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
                navigate("/connect");
              }}
            />
          </Box>

          <Group>
            <a
              className={classes.link}
              data-active={
                window.location.hash.includes("connect") ||
                window.location.hash.includes("main_window") || undefined
              }
              href=""
              key={"Tor"}
              onClick={(event) => {
                event.preventDefault();
                setActive("Connect");
                try {
                  navigate("/connect");
                } catch (e) {}
              }}
            >
              <IconCircuitBattery className={classes.linkIcon} stroke={1.5} />
              <span>Connect to El Tor</span>
            </a>
            <a
              className={classes.link}
              data-active={window.location.hash.includes("relay") || undefined}
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
              data-active={window.location.hash.includes("wallet") || undefined}
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
      <Container mt="md">
        {/* Main Content Renders here in Oulet */}
        <Outlet />
      </Container>
     
    </Container>
  );
}
