import { Stack, Title } from "@mantine/core";
import { useLocalStorage } from "usehooks-ts";
import { type WalletProviderType } from "../services/LocalStorage"

export const Wallet = () => {

  const [defaultWallet, setDefaultWallet, removeDefaultWallet] = useLocalStorage("defaultWallet", "None" as WalletProviderType);
  
  return (
    <Stack>
      <Title order={2}>Wallet</Title>
      <Title order={4}>Default Wallet: {defaultWallet}</Title>
    </Stack>
  );
};
