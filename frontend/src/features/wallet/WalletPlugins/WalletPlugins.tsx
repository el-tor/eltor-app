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

import phoenixDLogo from "../../../features/wallet/phoenixDLogo.svg";
import lndLogo from "../../../features/wallet/lndLogo.svg";
import clnLogo from "../../../features/wallet/clnLogo.svg";
import strikeLogo from "../../../features/wallet/strikeLogo.svg";

import { WalletBox } from "./WalletBox";

// export const WalletPlugins = ({ setShowWallet, showWallet }) => {
export const WalletPlugins = ({ }) => {
  return (
    <Group gap="0">
      <WalletBox
        logo={phoenixDLogo}
        onClick={() => {
          // setShowWallet(!showWallet);
        }}
        isDefault={true}
      />
      <WalletBox
        logo={clnLogo}
        onClick={() => {
          alert("todo implement");
        }}
      />
      <WalletBox
        logo={lndLogo}
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
