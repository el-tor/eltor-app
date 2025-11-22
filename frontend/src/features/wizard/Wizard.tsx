import {
  Group,
  TextInput,
  Checkbox,
  Button,
  Anchor,
  Text,
  PasswordInput,
  Modal,
  Alert,
  Box,
  Title,
  Center,
  Collapse,
} from '@mantine/core'
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useDispatch, useSelector } from '../../hooks'
import { setRelayEnabled } from '../../globalStore'
import {
  IconInfoCircleFilled,
  IconSquareNumber1,
  IconSquareNumber2,
  IconSquareNumber3,
  IconSettingsCheck,
  IconQuestionMark,
  IconCurrencyBitcoin,
  IconArrowNarrowRight,
  IconArrowNarrowLeft,
} from '@tabler/icons-react'

export function Wizard({ close }: { close: () => void }) {
  const [step, setStep] = useState(1)
  const navigate = useNavigate()
  const dispatch = useDispatch()
  const relayEnabled = useSelector((state) => state.global.relayEnabled)

  return (
    <Modal.Root opened={true} onClose={close} size="40rem">
      <Modal.Overlay />
      <Modal.Content>
        <Modal.Body>
          <Box m="xs">
            {step === 1 && (
              <>
                <Group>
                  <Title order={3}>Welcome to El Tor</Title>
                  <Text ml="auto">1 / 3</Text>
                </Group>
                <Group align="flex-start" mb="lg" mt="lg">
                  <IconSettingsCheck
                    color="rgb(245, 54, 245)"
                    size={20}
                    style={{ marginTop: 2 }}
                  />
                  <Text style={{ flex: 1 }}>
                    First we need to configure a few things. Let's get started!
                  </Text>
                </Group>

                <Group justify="flex-end" mt="xl">
                  <Button onClick={() => setStep(2)}>
                    Next
                    <IconArrowNarrowRight
                      size={16}
                      style={{ marginLeft: 4 }}
                    />{' '}
                  </Button>
                </Group>
              </>
            )}
            {step === 2 && (
              <>
                <Group>
                  <Title order={3}>Configure Tor</Title>
                  <Text ml="auto">2 / 3</Text>
                </Group>
                <Box mt="lg">
                  <Group align="flex-start" mb="lg" mt="lg">
                    <IconQuestionMark
                      color="rgb(245, 54, 245)"
                      size={20}
                      style={{ marginTop: 2 }}
                    />
                    <Text style={{ flex: 1 }}>
                      Do you want to run a relay and get paid for sharing your
                      bandwidth? It is perfectly fine not to run a relay and
                      simply use El Tor as a Client.
                    </Text>
                  </Group>
                  <Group align="flex-start" mb="lg" mt="lg" ml="32">
                    <Checkbox
                      label="Yes, I want to run a relay"
                      mt="lg"
                      onChange={(event) => {
                        if (event.currentTarget.checked) {
                          dispatch(setRelayEnabled(true))
                        } else {
                          dispatch(setRelayEnabled(false))
                        }
                      }}
                    />
                  </Group>

                  <Collapse in={relayEnabled} mt="xl" ml="32">
                    <Text color="dimmed">
                      <IconInfoCircleFilled size="16" /> Running a relay
                      requires some extra configuration, like making sure ports
                      are allowed thru your firewall. See "Run a Relay" for
                      configuration details after the wizard.
                    </Text>
                  </Collapse>
                </Box>
                <Group mt="xl">
                  <Button onClick={() => setStep(1)}>
                    <IconArrowNarrowLeft size={16} style={{ marginRight: 4 }} />
                    Back
                  </Button>
                  <Button ml="auto" onClick={() => setStep(3)}>
                    Next
                    <IconArrowNarrowRight
                      size={16}
                      style={{ marginLeft: 4 }}
                    />{' '}
                  </Button>
                </Group>
              </>
            )}
            {step === 3 && (
              <Box>
                <Group>
                  <Title order={3}>Configure Lightning Node</Title>
                  <Text ml="auto">3 / 3</Text>
                </Group>
                <Box mt="lg">
                  <Group align="flex-start" mb="xs">
                    <IconSquareNumber1
                      color="rgb(245, 54, 245)"
                      size={20}
                      style={{ marginTop: 2 }}
                    />
                    <Text style={{ flex: 1 }}>
                      On the next screen, configure your lightning node (Core
                      Lightning or Phoenixd).
                    </Text>
                  </Group>

                  <Group align="flex-start" mb="xs">
                    <IconSquareNumber2
                      color="rgb(245, 54, 245)"
                      size={20}
                      style={{ marginTop: 2 }}
                    />
                    <Text style={{ flex: 1 }}>
                      If you do not have a node you can choose Phoenixd and the
                      app will locally spin up an embedded Phoenixd node for
                      you. Just load it up with some sats!
                    </Text>
                  </Group>

                  <Group align="flex-start" mb="xl">
                    <IconSquareNumber3
                      color="rgb(245, 54, 245)"
                      size={20}
                      style={{ marginTop: 2 }}
                    />
                    <Text style={{ flex: 1 }}>
                      After that you will be all set to use El Tor by clicking "Activate" on the Home page!
                    </Text>
                  </Group>
                </Box>
                <Group>
                  <Button onClick={() => setStep(2)}>
                    <IconArrowNarrowLeft size={16} style={{ marginRight: 4 }} />
                    Back
                  </Button>
                  <Button
                    ml="auto"
                    onClick={() => {
                      navigate('/wallet')
                    }}
                  >
                    Configure
                  </Button>
                </Group>
              </Box>
            )}
          </Box>
        </Modal.Body>
      </Modal.Content>
    </Modal.Root>
  )
}
