import { useEffect } from "react";
import { Route, useLocation } from "wouter";
import { Connect } from "./Connect";
import { Wallet } from "./Wallet";
import { Relay } from "./Relay";

export const Router = () => {
  const [, navigate] = useLocation();

  // Handle tray clicks
  // the rest is configured in preload.ts and index.ts
  useEffect(() => {
    // Connect
    (window as any).electron.onNavigateToConnect(() => {
      navigate("/connect/true");
      (window as any).electron.menuActivateConnect();
    });
    (window as any).electron.onNavigateToDeactivateConnect(() => {
      navigate("/connect/false");
      (window as any).electron.menuDeactivateConnect();
    });

    // Relay
    (window as any).electron.onNavigateToRelay(() => {
      navigate("/relay");
      (window as any).electron.menuActivateRelay();
    });
    (window as any).electron.onNavigateToDeactivateRelay(() => {
      navigate("/relay");
      (window as any).electron.menuDeactivateRelay();
    });

    // Wallet
    (window as any).electron.onNavigateToWallet(() => {
      navigate("/wallet");
    });
  }, [navigate]);

  useEffect(() => {
    navigate("/connect/false");
  }, []);

  return (
    <>
      <Route path="/main_window/:connected" component={Connect} />
      <Route path="/connect/:connected" component={Connect} />
      <Route path="/relay" component={Relay} />
      <Route path="/wallet" component={Wallet} />
      {/* add more routes here */}
    </>
  );
};
