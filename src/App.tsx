import { createRoot } from "react-dom/client";
import { Router } from "./pages/Router";
import Nav from "./components/Nav";
import { createTheme, MantineProvider, Container, Stack } from "@mantine/core";
import "@mantine/core/styles.css"; // This imports the base styles for Mantine core components
import "./globals.css";

const theme = createTheme({
  // Define your theme here
  components: {},
});

const root = createRoot(document.body);
root.render(
  <MantineProvider theme={theme} forceColorScheme="dark">
    <Nav />
    <Container mt="sm" w="768px" h="520px">
      <Stack align="center" gap="xs">
        <Router />
      </Stack>
    </Container>
  </MantineProvider>
);
