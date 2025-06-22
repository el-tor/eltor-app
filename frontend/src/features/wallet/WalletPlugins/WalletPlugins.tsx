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

import phoenixDLogo from "../phoenixdLogo.svg";
import lndLogo from "../lndLogo.svg";
import clnLogo from "../clnLogo.svg";
import strikeLogo from "../strikeLogo.svg";

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
