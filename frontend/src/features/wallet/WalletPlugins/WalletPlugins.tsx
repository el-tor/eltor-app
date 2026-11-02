import { Group } from '@mantine/core'
import phoenixDLogo from '../phoenixdLogo.svg'
import lndLogo from '../lndLogo.svg'
import clnLogo from '../clnLogo.svg'
import strikeLogo from '../strikeLogo.svg'
import { WalletBox } from './WalletBox'
import { useDispatch } from '../../../hooks'
import {
  setDefaultWallet,
  setClickedWallet,
} from './../walletStore'

// export const WalletPlugins = ({ setShowWallet, showWallet }) => {
export const WalletPlugins = ({ defaultWallet, walletReady, onClick }: { defaultWallet: string, walletReady: boolean, onClick: () => void }) => {
  const dispatch = useDispatch()
  return (
    <Group gap="0">
      <WalletBox
        logo={phoenixDLogo}
        onClick={() => {
          dispatch(setClickedWallet('phoenixd') )
          onClick()
        }}
        ready={walletReady}
        isDefault={defaultWallet === 'phoenixd'}
      />
      <WalletBox
        logo={clnLogo}
        onClick={() => {
          dispatch(setClickedWallet('cln'))
          onClick()
        }}
        ready={walletReady}
        isDefault={defaultWallet === 'cln'}
      />
      <WalletBox
        logo={lndLogo}
        onClick={() => {
          dispatch(setClickedWallet('lnd'))
          onClick()
        }}
        ready={walletReady}
        isDefault={defaultWallet === 'lnd'}
      />
      <WalletBox
        logo={strikeLogo}
        onClick={() => {
          dispatch(setClickedWallet('strike'))
          onClick()
        }}
        ready={walletReady}
        isDefault={defaultWallet === 'strike'}
      />
    </Group>
  )
}
