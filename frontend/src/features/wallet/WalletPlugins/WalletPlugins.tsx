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
export const WalletPlugins = ({ defaultWallet, onClick }: { defaultWallet: string, onClick: () => void }) => {
  const dispatch = useDispatch()
  return (
    <Group gap="0">
      <WalletBox
        logo={phoenixDLogo}
        onClick={() => {
          dispatch(setClickedWallet('phoenixd') )
          onClick()
        }}
        isDefault={defaultWallet === 'phoenixd'}
      />
      <WalletBox
        logo={clnLogo}
        onClick={() => {
          dispatch(setClickedWallet('cln'))
          onClick()
        }}
        isDefault={defaultWallet === 'cln'}
      />
      <WalletBox
        logo={lndLogo}
        onClick={() => {
          dispatch(setClickedWallet('lnd'))
          onClick()
        }}
        isDefault={defaultWallet === 'lnd'}
      />
      <WalletBox
        logo={strikeLogo}
        onClick={() => {
          dispatch(setClickedWallet('strike'))
          onClick()
        }}
        isDefault={defaultWallet === 'strike'}
      />
    </Group>
  )
}
