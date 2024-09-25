import { createRoot } from "react-dom/client";
import { Router } from "./pages/Router";
import "./globals.css";
import Nav from "./components/Nav";
import { createTheme, MantineProvider } from "@mantine/core";

const theme = createTheme({
  // add theme
});

const root = createRoot(document.body);
root.render(
  <MantineProvider theme={theme} withGlobalStyles withNormalizeCSS>
    <div
      style={{
        width: "100%",
        height: "100%",
        backgroundColor: "rgb(26, 26, 26)",
      }}
    >
      <Nav />
      <Router />
    </div>
  </MantineProvider>
);
