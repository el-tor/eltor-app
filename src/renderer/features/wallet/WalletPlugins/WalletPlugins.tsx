import {
  Stack,
  Title,
  Center,
  Select,
  Button,
  Box,
  Loader,
  Text,
  Group,
} from "@mantine/core";

import { ChannelBalanceLine } from "renderer/components/ChannelBalanceLine";
import phoenixDLogo from "renderer/features/wallet/phoenixDLogo.svg";
import lndLogo from "renderer/features/wallet/lndLogo.svg";
import clnLogo from "renderer/features/wallet/clnLogo.svg";
import strikeLogo from "renderer/features/wallet/strikeLogo.svg";


import { WalletBox } from "./WalletBox";

export const WalletPlugins = ({ setShowWallet, showWallet }) => {
  return (
    <Group>
      <WalletBox
        logo={phoenixDLogo}
        onClick={() => {
          // setShowWallet(!showWallet);
        }}
        isDefault={true}
      />
      <WalletBox
        logo={lndLogo}
        onClick={() => {
          alert("todo implement");
        }}
      />
      <WalletBox
        logo={clnLogo}
        onClick={() => {
          alert("todo implement");
        }}
      />
       <WalletBox
        logo={strikeLogo}
        onClick={() => {
          alert("todo implement");
        }}
      />
    </Group>
  );
};
