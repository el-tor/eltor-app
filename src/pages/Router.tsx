import { useEffect } from "react";
import { Route, useLocation } from "wouter";
import { Connect } from "./Connect";
import { Wallet } from "./Wallet";
import { Relay } from "./Relay";
import { useLocalStorage } from "usehooks-ts";

export const Router = () => {
  const [, navigate] = useLocation();
  const [torActive, setTorActive, removeTorActive] = useLocalStorage("torActive", "false");
  const [relayActive, setRelayActive, removeRelayActive] = useLocalStorage("relayActive", "false");



  // Handle tray clicks
  // the rest is configured in preload.ts and index.ts
  useEffect(() => {
    // Connect
    (window as any).electron.onNavigateToConnect(() => {
      navigate("/connect");
      (window as any).electron.menuActivateConnect();
      setTorActive("true");
    });
    (window as any).electron.onNavigateToDeactivateConnect(() => {
      navigate("/connect");
      (window as any).electron.menuDeactivateConnect();
      setTorActive("false");
    });

    // Relay
    (window as any).electron.onNavigateToRelay(() => {
      navigate("/relay");
      (window as any).electron.menuActivateRelay();
      setRelayActive("true");
      
    });
    (window as any).electron.onNavigateToDeactivateRelay(() => {
      navigate("/relay");
      (window as any).electron.menuDeactivateRelay();
      setRelayActive("false");
    });

    // Wallet
    (window as any).electron.onNavigateToWallet(() => {
      navigate("/wallet");
    });
  }, [navigate]);

  useEffect(() => {
    navigate("/connect");
  }, []);

  return (
    <>
      <Route path="/main_window" component={Connect} />
      <Route path="/connect" component={Connect} />
      <Route path="/relay" component={Relay} />
      <Route path="/wallet" component={Wallet} />
      {/* add more routes here */}
    </>
  );
};
