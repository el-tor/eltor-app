import { createRoot } from "react-dom/client";
import { Router } from "./pages/Router";
import Nav from "./components/Nav";
import { createTheme, MantineProvider, Container, Stack } from "@mantine/core";
import "@mantine/core/styles.css"; // This imports the base styles for Mantine core components
import "./globals.css";
import { LocalStorage } from "./services/LocalStorage";

const theme = createTheme({
  // Define your theme here
  components: {},
});

// Target the specific container, not document.body
const rootElement = document.getElementById('root');
// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const root = createRoot(rootElement!);

root.render(
  <LocalStorage>
    <MantineProvider theme={theme} forceColorScheme="dark">
      <Nav />
      <Container mt="sm" w="768px" h="520px">
        <Stack align="center" gap="xs">
          <Router />
        </Stack>
      </Container>
    </MantineProvider>
  </LocalStorage>
);
