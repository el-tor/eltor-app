import {
  Stack,
  Title,
  Center,
  Select,
  Button,
  Box,
  Loader,
  Text,
} from "@mantine/core";
import { useDispatch, useSelector } from "../../hooks";
import { WalletProviderType } from "renderer/drivers/IWallet";
import { useEffect, useState } from "react";
import {
  fetchChannelInfo,
  fetchWalletBalance,
  setDefaultWallet,
  getBolt12Offer,
} from "./walletStore";
import { ChannelBalanceLine } from "renderer/components/ChannelBalanceLine";
import { WalletPlugins } from "./WalletPlugins/WalletPlugins";
import { get } from "http";
import CopyableTextBox from "renderer/components/CopyableTextBox";

export const Wallet = () => {
  const {
    balance,
    defaultWallet,
    channelInfo,
    bolt12Offer,
    requestState,
    error,
    loading,
  } = useSelector((state) => state.wallet);
  const dispatch = useDispatch();
  const [showWallet, setShowWallet] = useState(true);

  fetchChannelInfo("");
  fetchChannelInfo("");

  useEffect(() => {
    dispatch(fetchWalletBalance(""));
    dispatch(fetchChannelInfo(""));
    dispatch(getBolt12Offer(""));
  }, []);

  return (
    <Stack>
      {showWallet && (
        <Box>
          <Center mb="md">
            <Title order={2}>{defaultWallet} Wallet</Title>
          </Center>

          <Box>
            <Title order={4}>Balance: {balance}</Title>
            <ChannelBalanceLine
              send={balance ?? 0}
              receive={channelInfo.receive ?? 0}
            />
          </Box>
          <Box mt="lg">
            <Title order={5}>BOLT 12 Offer</Title>
            <CopyableTextBox text={bolt12Offer} />
          </Box>

          <Loader display={loading ? "block" : "none"} />
          {/* <Button
            w="100%"
            display={loading ? "none" : "block"}
            onClick={() => {
              dispatch(fetchWalletBalance(""));
            }}
          >
            Get Balance
          </Button> */}

          <Stack mt="xl">
            <Center>
              <WalletPlugins
                setShowWallet={setShowWallet}
                showWallet={showWallet}
              />
            </Center>
            {/* <Select
              label="Change your default wallet"
              placeholder=""
              value={defaultWallet}
              onChange={(value) => {
                dispatch(setDefaultWallet(value as WalletProviderType));
              }}
              data={["Phoenix", "Lndk", "CoreLightning", "None"]}
            /> */}
          </Stack>
        </Box>
      )}
    </Stack>
  );
};
