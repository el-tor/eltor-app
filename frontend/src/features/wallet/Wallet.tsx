import {
  Modal,
  Title,
  Center,
  Box,
  Loader,
  Group,
  SimpleGrid,
  Checkbox,
  Image,
} from '@mantine/core'
import { useDispatch, useSelector } from '../../hooks'
import { useEffect, useState } from 'react'
import { setDefaultWallet, getBolt12Offer, fetchNodeInfo } from './walletStore'
import { ChannelBalanceLine } from '../../components/ChannelBalanceLine'
import { WalletPlugins } from './WalletPlugins/WalletPlugins'
import CopyableTextBox from '../../components/CopyableTextBox'
import QRCode from 'react-qr-code'
import { IconRefresh } from '@tabler/icons-react'
import { Circle } from '../../components/Circle'
import { Transactions } from './Transactions'
import { useDisclosure } from '@mantine/hooks'
import { WalletConfigModal } from './WalletConfigModal'
import phoenixDLogo from './phoenixdLogo.svg'
import lndLogo from './lndLogo.svg'
import clnLogo from './clnLogo.svg'
import strikeLogo from './strikeLogo.svg'
import styles from './WalletPlugins/WalletBox.module.css'
import { clear } from 'console'

export interface IWallet {
  getWalletTransactions: (walletId: string) => Promise<any>
  payInvoice: (invoice: string) => Promise<string>
  getBolt12Offer: () => Promise<string>
  fetchWalletBalance: () => Promise<FetchWalletBalanceResponseType>
  decodeInvoice: (invoice: string) => Promise<any>
  checkPaymentStatus: (paymentId: string) => Promise<any>
  fetchChannelInfo: (channelId: string) => Promise<FetchChannelInfoResponseType>
  onPaymentReceived: (event: any) => void
}

export type {
  FetchWalletBalanceResponseType,
  WalletProviderType,
  FetchChannelInfoResponseType,
}

type FetchWalletBalanceResponseType = {
  balance: number
}

type FetchChannelInfoResponseType = {
  send: number
  receive: number
}

type WalletProviderType = 'phoenixd' | 'lnd' | 'cln' | 'strike' | 'none'

export const Wallet = () => {
  const {
    send,
    receive,
    defaultWallet,
    clickedWallet,
    channelInfo,
    bolt12Offer,
    requestState,
    error,
    loading,
  } = useSelector((state) => state.wallet)
  const dispatch = useDispatch()
  const [showWallet, setShowWallet] = useState(true)
  const [opened, { open, close }] = useDisclosure(false)

  useEffect(() => {
    dispatch(fetchNodeInfo(''))
    dispatch(getBolt12Offer(''))
  }, [])

  return (
    <Box>
      {showWallet && (
        <Box w="100%">
          <Modal.Root opened={opened} onClose={close} size="40rem">
            <Modal.Overlay />
            <Modal.Content>
              <Modal.Header>
                <Group justify="space-between" align="top" w="100%">
                  <Box
                    w={160}
                    h={52}
                    m="xs"
                    className={styles.box2}
                    bg="white"
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      justifyContent: 'center',
                      alignItems: 'center',
                      position: 'relative',
                    }}
                  >
                    <Center style={{ width: '100%', height: '100%' }}>
                      <Image
                        bg="white"
                        src={(() => {
                          switch (clickedWallet) {
                            case 'phoenixd':
                              return phoenixDLogo
                            case 'lnd':
                              return lndLogo
                            case 'cln':
                              return clnLogo
                            case 'strike':
                              return strikeLogo
                            default:
                              return ''
                          }
                        })()}
                        alt={`${clickedWallet} Logo`}
                        h="26px"
                      />
                    </Center>
                  </Box>
                  <Modal.CloseButton />
                </Group>
              </Modal.Header>
              <Modal.Body>
                <WalletConfigModal />
              </Modal.Body>
            </Modal.Content>
          </Modal.Root>
          <Group>
            <Center>
              <WalletPlugins defaultWallet={defaultWallet} onClick={open} />
            </Center>
            <Group ml="auto">
              <Center>
                {' '}
                {loading && (
                  <Loader
                    size="sm"
                    style={{
                      visibility: loading ? 'visible' : 'hidden',
                    }}
                  />
                )}
              </Center>
            </Group>
            {!!!defaultWallet && <Circle color={'#FF6347'} />}
          </Group>

          <Group mt="xl">
            <Title order={4}>
              Balance:{' '}
              <span style={{ fontFamily: 'monospace' }}>
                {send?.toLocaleString()}
              </span>
            </Title>
            <IconRefresh
              stroke={1.5}
              onClick={() => {
                dispatch(fetchNodeInfo(''))
              }}
              style={{ cursor: 'pointer' }}
            />
          </Group>
          <ChannelBalanceLine send={send ?? 0} receive={receive ?? 0} />

          <SimpleGrid
            mt="lg"
            cols={{ base: 1, sm: 2 }} // Stack vertically on small screens, two columns on larger screens
            spacing={{ base: 'md', sm: 'lg' }} // Adjust spacing based on screen size
            verticalSpacing={{ base: 'md', sm: 'lg' }} // Adjust vertical spacing based on screen size
          >
            <Box bg="white" p="md" style={{ borderRadius: '6px' }}>
              <Center>
                <Title order={5} mb="xs" style={{ color: 'black' }}>
                  BOLT 12 Offer
                </Title>
              </Center>
              <Center>
                <QRCode
                  value={bolt12Offer}
                  size={280}
                  style={{ border: 2, borderColor: 'whitesmoke' }}
                />
              </Center>
              <CopyableTextBox text={bolt12Offer} limitChars={36} bg="white" />
            </Box>

            <Transactions h="450px" />
          </SimpleGrid>
          <Checkbox mt="xl" defaultChecked label="Default Wallet" />
        </Box>
      )}
    </Box>
  )
}
