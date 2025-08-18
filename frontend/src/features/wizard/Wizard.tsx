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
} from '@mantine/core'
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useDispatch, useSelector } from '../../hooks'
import { setRelayEnabled } from '../../globalStore'

export function Wizard({ close }: { close: () => void }) {
  const [step, setStep] = useState(1)
  const navigate = useNavigate()
  const dispatch = useDispatch()

  return (
    <Modal.Root opened={true} onClose={close} size="40rem">
      <Modal.Overlay />
      <Modal.Content>
        <Modal.Body>
          <Box m="xs">
            {step === 1 && (
              <>
                <Title order={3}>Welcome to El Tor (1/3)</Title>
                <Box m="xs">
                  <Text>First we need to configure a few things!</Text>
                </Box>
                <Group justify="flex-end">
                  <Button onClick={() => setStep(2)}>Next</Button>
                </Group>
              </>
            )}
            {step === 2 && (
              <>
                <Title order={3}>Configure Tor (2/3)</Title>
                <Box m="xs">
                  <p>
                    <Text>
                      Do you want to run a relay and get paid for sharing your
                      bandwidth? *
                      <p>
                        <i>
                          <Text size="sm">
                            *This requires some configuration like making sure
                            ports are allowed thru your firewall. See "Run a
                            Relay" for config
                          </Text>
                        </i>
                      </p>
                      <p>
                        It is perfectly fine not to run a relay and simply use
                        El Tor as a Client.
                      </p>
                    </Text>
                    <Checkbox
                      label="Yes, I want to run a relay"
                      mt="sm"
                      onChange={(event) => {
                        if (event.currentTarget.checked) {
                          dispatch(setRelayEnabled(true))
                        } else {
                          dispatch(setRelayEnabled(false))
                        }
                      }}
                    />
                  </p>
                  <p></p>
                </Box>
                <Group justify="flex-end">
                  <Button onClick={() => setStep(1)}>Back</Button>
                  <Button onClick={() => setStep(3)}>Next</Button>
                </Group>
              </>
            )}
            {step === 3 && (
              <>
                <Title order={3}>Configure Lightning Node (3/3)</Title>
                <Box m="xs">
                  <p>
                    On the next screen configure your lightning node (Core
                    Lightning, LND, Strike or Phoenixd).
                  </p>
                  <p>
                    If you do not have a node you can choose Phoenixd and the
                    app will locally spin up an embedded Phoenixd node for you
                    (just load it up with sats!).
                  </p>
                  <p>After that you will be all set to use El Tor!</p>
                </Box>
                <Group justify="flex-end">
                  <Button onClick={() => setStep(2)}>Back</Button>
                  <Button
                    onClick={() => {
                      navigate('/wallet')
                    }}
                  >
                    Next
                  </Button>
                </Group>
              </>
            )}
          </Box>
        </Modal.Body>
      </Modal.Content>
    </Modal.Root>
  )
}
