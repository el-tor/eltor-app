import { useEffect } from "react";
import { Route, useLocation } from "wouter";

import { Connect } from "./Connect";
import { Wallet } from "./Wallet";
import { Relay } from "./Relay";
import { useLocalStorage } from "usehooks-ts";

declare let window: any;

export const Router = () => {
  const [, navigate] = useLocation();
  const [torActive, setTorActive, removeTorActive] = useLocalStorage(
    "torActive",
    "false"
  );
  const [relayActive, setRelayActive, removeRelayActive] = useLocalStorage(
    "relayActive",
    "false"
  );

  // Handle tray clicks
  // the rest is configured in preload.ts and index.ts
  useEffect(() => {
    // Connect
    window.electron.onNavigateToConnect(() => {
      navigate("/connect");
      window.electron.menuActivateConnect();
      setTorActive("true");
    });
    window.electron.onNavigateToDeactivateConnect(() => {
      navigate("/connect");
      window.electron.menuDeactivateConnect();
      setTorActive("false");
    });

    // Relay
    window.electron.onNavigateToRelay(() => {
      navigate("/relay");
      window.electron.menuActivateRelay();
      setRelayActive("true");
    });
    window.electron.onNavigateToDeactivateRelay(() => {
      navigate("/relay");
      window.electron.menuDeactivateRelay();
      setRelayActive("false");
    });

    // Wallet
    window.electron.onNavigateToWallet(() => {
      navigate("/wallet");
    });
  }, []);

  return (
    <>
      <Route path="/main_window" component={Connect} />
      <Route path="/connect" component={Connect} />
      <Route path="/relay" component={Relay} />
      <Route path="/wallet" component={Wallet} />
      <Route path="/" component={Connect} />
      <Route path="index.html" component={Connect} />
      {/* add more routes here */}
    </>
  );
};
