import {
  Stack,
  Title,
  Center,
  Select,
  Button,
  Box,
  Loader,
} from "@mantine/core";
import { useDispatch, useSelector } from "../../hooks";
import { WalletProviderType } from "renderer/drivers/IWallet";
import { useEffect } from "react";
import {
  fetchChannelInfo,
  fetchWalletBalance,
  setDefaultWallet,
} from "./walletSlice";
import { ChannelBalanceLine } from "renderer/components/ChannelBalanceLine";

export const Wallet = () => {
  const { balance, defaultWallet, channelInfo, requestState, error, loading } =
    useSelector((state) => state.wallet);
  const dispatch = useDispatch();

  fetchChannelInfo("");
  fetchChannelInfo("");

  useEffect(() => {
    dispatch(fetchWalletBalance(""));
    dispatch(fetchChannelInfo(""));
  }, []);

  return (
    <Stack>
      <Title order={4}>Default Wallet: {defaultWallet}</Title>
      <Select
        label="Choose your default wallet"
        placeholder=""
        value={defaultWallet}
        onChange={(value) => {
          dispatch(setDefaultWallet(value as WalletProviderType));
        }}
        data={["Phoenix", "Lndk", "CoreLightning", "None"]}
      />
      <Center>
        <Loader display={loading ? "block" : "none"} />
        <Button
          w="100%"
          display={loading ? "none" : "block"}
          onClick={() => {
            dispatch(fetchWalletBalance(""));
          }}
        >
          Get Balance
        </Button>
      </Center>
      <Title order={4}>Balance: {balance}</Title>
      <ChannelBalanceLine
        send={channelInfo.receive ?? 0}
        receive={channelInfo.receive ?? 0}
      />
    </Stack>
  );
};
