import ReactDom from "react-dom/client";
import React from "react";
import "@mantine/core/styles.css";
import { Routes } from "./routes";
import "./globals.css";
import { createTheme, MantineProvider } from "@mantine/core";
import { LocalStorage } from "./services/LocalStorage";

const theme = createTheme({
  // Define your theme here
  components: {},
});

ReactDom.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <LocalStorage>
      <MantineProvider theme={theme} forceColorScheme="dark">
        <Routes />
      </MantineProvider>
    </LocalStorage>
  </React.StrictMode>
);
