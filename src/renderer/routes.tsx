import { Route } from "react-router-dom";
import { Router } from "lib/electron-router-dom";
import { Layout } from "./layout";
import { Connect } from "./screens/Connect";
import { Relay } from "./screens/Relay";
import { WalletPage } from "./screens/WalletPage";

export function Routes() {
  return (
    <Router
      main={
        <Route path="/" element={<Layout />}>
          <Route path="/" element={<Connect />} />
          <Route path="/main_window" element={<Connect />} />
          <Route path="index.html" element={<Connect />} />
          <Route path="/connect" element={<Connect />} />
          <Route path="/relay" element={<Relay />} />
          <Route path="/wallet" element={<WalletPage />} />
        </Route>
      }
    />
  );
}
