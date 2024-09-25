import { useEffect } from "react";
import { Route, useLocation } from "wouter";
import { Home } from "./Home";
import { Wallet } from "./Wallet";

export const Router = () => {
  const [, navigate] = useLocation();

  // Handle tray clicks
  // the rest is configured in preload.ts and index.ts
  useEffect(() => {
    (window as any).electron.onNavigateToWallet(() => {
      navigate("/wallet");
    });
  }, [navigate]);

  return (
    <>
      <Route path="/" component={Home} />
      <Route path="/main_window" component={Home} />
      <Route path="/wallet" component={Wallet} />
      <Route path="/connect" component={Home} />
      <Route path="/host" component={Wallet} />
      {/* add more routes here */}
    </>
  );
};
