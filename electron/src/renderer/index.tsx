import ReactDom from "react-dom/client";
import React from "react";
import "@mantine/core/styles.css";
import { Routes } from "./routes";
import "./globals.css";
import { Button, createTheme, MantineProvider } from "@mantine/core";
import { Provider } from "react-redux";
import { store, persistor } from "./store";
import { PersistGate } from "redux-persist/integration/react";
import { theme } from "./theme";

ReactDom.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <MantineProvider theme={theme} forceColorScheme="dark">
          <Routes />
        </MantineProvider>
      </PersistGate>
    </Provider>
  </React.StrictMode>
);
