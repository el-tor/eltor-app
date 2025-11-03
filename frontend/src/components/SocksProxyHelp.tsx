import { Modal, ActionIcon, Stack, Box, Text, ScrollArea, Tooltip, Anchor } from '@mantine/core'
import { useDisclosure } from '@mantine/hooks'
import { IconHelp } from '@tabler/icons-react'

interface SocksProxyHelpProps {
  hostname: string
  port: number | string
}

export const SocksProxyHelp = ({ hostname, port }: SocksProxyHelpProps) => {
  const [opened, { open, close }] = useDisclosure(false)

  return (
    <>
      <Anchor ml="xs" size="sm" onClick={open} underline="always" c="white">Help</Anchor>
      <Tooltip label="Click for socks proxy setup instructions" position="top" withArrow>
        <ActionIcon
          variant="subtle"
          size="sm"
          color="gray"
          onClick={open}
        >
          <IconHelp size={16} />
        </ActionIcon>
      </Tooltip>

      <Modal
        opened={opened}
        onClose={close}
        title="SOCKS5 Proxy Setup Instructions"
        size="lg"
        centered
      >
        <ScrollArea h={500} type="auto">
          <Stack gap="md" pr="md">
            <Box>
              <Text fw={600} mb={4} c="blue">
                🦊 Firefox Setup:
              </Text>
              <Text size="sm">
                1. Open Firefox Settings → Network Settings → Settings
                <br />
                2. Select "Manual proxy configuration"
                <br />
                3. SOCKS Host:{' '}
                <Text span fw={600}>
                  {hostname}
                </Text>
                <br />
                4. Port:{' '}
                <Text span fw={600}>
                  {port}
                </Text>
                <br />
                5. Select "SOCKS v5"
                <br />
                6. Check "Proxy DNS when using SOCKS v5"
                <br />
                7. Click OK
              </Text>
            </Box>

            <Box>
              <Text fw={600} mb={4} c="red">
                🌐 Chrome Setup:
              </Text>
              <Text size="sm">
                Chrome uses system proxy settings by default.
                <br />
                Or launch with command line:
                <br />
                <Text span fw={600} style={{ fontFamily: 'monospace' }}>
                  chrome --proxy-server="socks5://{hostname}:{port}"
                </Text>
              </Text>
            </Box>

            <Box>
              <Text fw={600} mb={4} c="grape">
                🍎 macOS System-Wide Proxy:
              </Text>
              <Text size="sm">
                1. Open System Settings → Network
                <br />
                2. Select your active network connection
                <br />
                3. Click "Details" → "Proxies"
                <br />
                4. Enable "SOCKS Proxy"
                <br />
                5. Server:{' '}
                <Text span fw={600}>
                  {hostname}
                </Text>
                <br />
                6. Port:{' '}
                <Text span fw={600}>
                  {port}
                </Text>
                <br />
                7. Click OK
              </Text>
            </Box>

            <Box>
              <Text fw={600} mb={4} c="cyan">
                🪟 Windows System-Wide Proxy:
              </Text>
              <Text size="sm">
                1. Open Settings → Network & Internet → Proxy
                <br />
                2. Under "Manual proxy setup", click "Edit"
                <br />
                3. Enable "Use a proxy server"
                <br />
                4. In "Proxy IP address":{' '}
                <Text span fw={600}>
                  {hostname}
                </Text>
                <br />
                5. In "Port":{' '}
                <Text span fw={600}>
                  {port}
                </Text>
                <br />
                6. Check "Don't use the proxy server for local addresses"
                <br />
                7. Click Save
              </Text>
            </Box>

            <Box>
              <Text fw={600} mb={4} c="orange">
                🐧 Linux System-Wide Proxy:
              </Text>
              <Text size="sm">
                <Text fw={600}>GNOME/Ubuntu:</Text>
                1. Open Settings → Network → Network Proxy
                <br />
                2. Select "Manual"
                <br />
                3. In "Socks Host":{' '}
                <Text span fw={600}>
                  {hostname}
                </Text>
                <br />
                4. In "Port":{' '}
                <Text span fw={600}>
                  {port}
                </Text>
                <br />
                5. Click Apply
                <br />
                <br />
                <Text fw={600}>KDE Plasma:</Text>
                1. System Settings → Network → Proxy
                <br />
                2. Enable "Manually specify proxy"
                <br />
                3. SOCKS proxy:{' '}
                <Text span fw={600}>
                  {hostname}:{port}
                </Text>
                <br />
                4. Click Apply
              </Text>
            </Box>
          </Stack>
        </ScrollArea>
      </Modal>
    </>
  )
}
