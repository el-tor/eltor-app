import React, { useEffect, useState } from "react";
import { Box, Group, Container, Center } from "@mantine/core";
import {
  IconWifi,
  IconCurrencyBitcoin,
  IconDeviceDesktop,
} from "@tabler/icons-react";
import eltorLogo from "./assets/eltor-logo.png";
import classes from "./globals.module.css";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
const { electronEvents } = window;
import styles from "./globals.module.css";
import { setTorActive, setRelayActive } from "./globalStore";
import { useDispatch } from "./hooks";

export function Layout() {
  const [active, setActive] = useState("Connect");
  const navigate = useNavigate();
  const [isLoaded, setIsLoaded] = useState(false);
  const dispatch = useDispatch();

  useEffect(() => {
    setIsLoaded(true);
  }, []);

  useEffect(() => {
    // Connect
    electronEvents.onNavigateToConnect(() => {
      navigate("/connect");
      electronEvents.menuActivateConnect(() => {});
      dispatch(setTorActive(true));
    });
    electronEvents.onNavigateToDeactivateConnect(() => {
      navigate("/connect");
      electronEvents.menuDeactivateConnect(() => {});
      dispatch(setTorActive(false));
    });

    // Relay
    electronEvents.onNavigateToRelay(() => {
      navigate("/relay");
      electronEvents.menuActivateRelay(() => {});
      dispatch(setRelayActive(true));
    });
    electronEvents.onNavigateToDeactivateRelay(() => {
      navigate("/relay");
      electronEvents.menuDeactivateRelay(() => {});
      dispatch(setRelayActive(false));
    });

    // Wallet
    electronEvents.onNavigateToWallet(() => {
      navigate("/wallet");
    });
  }, []);

  return (
    <Center>
      <Container
        w="90%"
        mt="sm"
        ml="xs"
        mr="xs"
        maw={styles.maxWidth}
        
        // bg="gray"
      >
        {isLoaded && (
          <Group>
            <Box ml="0">
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
            <Group align="center" ml="auto">
              <Group>
                <a
                  className={classes.link}
                  data-active={
                    window.location.hash.includes("connect") ||
                    window.location.hash.includes("main_window") ||
                    window.location.hash === "#/main" ||
                    undefined
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
                  <IconWifi
                    className={classes.linkIcon}
                    stroke={1.5}
                    color="rgb(245, 54, 245)"
                  />
                  <span style={{ color: "white" }}>Connect to El Tor</span>
                </a>
                <a
                  className={classes.link}
                  data-active={
                    window.location.hash.includes("relay") || undefined
                  }
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
                  <IconDeviceDesktop
                    className={classes.linkIcon}
                    stroke={1.5}
                    color="rgb(245, 54, 245)"
                  />
                  <span style={{ color: "white" }}>Run a Relay</span>
                </a>
                <a
                  className={classes.link}
                  data-active={
                    window.location.hash.includes("wallet") || undefined
                  }
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
                  <IconCurrencyBitcoin
                    className={classes.linkIcon}
                    stroke={1.5}
                    color="rgb(245, 54, 245)"
                  />
                  <span style={{ color: "white" }}>Wallet</span>
                </a>
              </Group>
            </Group>
          </Group>
        )}

        <Container mt="md">
          {/* Main Content Renders here in Outlet */}
          <Outlet />
        </Container>
      </Container>
    </Center>
  );
}
