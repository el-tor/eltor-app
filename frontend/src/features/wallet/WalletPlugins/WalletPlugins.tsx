import { Group } from '@mantine/core'

import phoenixDLogo from '../phoenixdLogo.svg'
import lndLogo from '../lndLogo.svg'
import clnLogo from '../clnLogo.svg'
import strikeLogo from '../strikeLogo.svg'

import { WalletBox } from './WalletBox'

// export const WalletPlugins = ({ setShowWallet, showWallet }) => {
export const WalletPlugins = ({ defaultWallet }: { defaultWallet: string }) => {
  return (
    <Group gap="0">
      <WalletBox
        logo={phoenixDLogo}
        onClick={() => {
          // refreshWallets();
        }}
        isDefault={defaultWallet === 'phoenixd'}
      />
      <WalletBox
        logo={clnLogo}
        onClick={() => {
          // refreshWallets();
        }}
        isDefault={defaultWallet === 'cln'}
      />
      <WalletBox
        logo={lndLogo}
        onClick={() => {
          // refreshWallets();
        }}
        isDefault={defaultWallet === 'lnd'}
      />
      <WalletBox
        logo={strikeLogo}
        onClick={() => {
          // refreshWallets();
        }}
        isDefault={defaultWallet === 'strike'}
      />
    </Group>
  )
}
