import { createRoot } from "react-dom/client";
import { Router } from "./pages/Router";
import Nav from "./components/Nav";
import { createTheme, MantineProvider, Box } from "@mantine/core";
import '@mantine/core/styles.css'; // This imports the base styles for Mantine core components
import "./globals.css";

const theme = createTheme({
  // Define your theme here
  
});

const root = createRoot(document.body);
root.render(
  <MantineProvider theme={theme} forceColorScheme="dark">
    <Box>
      <Nav />
      <Router />
    </Box>
  </MantineProvider>
);
