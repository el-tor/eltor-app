import { Stack, Title, Center, Select, Button } from "@mantine/core";
import { Wallet } from "renderer/features/wallet/Wallet";

export const WalletPage = () => {
  return (
    <Stack>
      <Center>
        <Title order={2}>Wallet</Title>
      </Center>
     <Wallet />
    </Stack>
  );
};
