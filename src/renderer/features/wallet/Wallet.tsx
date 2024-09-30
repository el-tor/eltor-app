import {
  Stack,
  Title,
  Center,
  Select,
  Button,
  Box,
  Loader,
} from "@mantine/core";
import { useSelector, useDispatch } from "react-redux";
import { setDefaultWallet, fetchWalletBalance, walletSlice } from "./walletSlice";
import { type RootState } from "renderer/store";
import { WalletProviderType } from "renderer/drivers/IWallet";
import { useEffect } from "react";

export const Wallet = () => {
  const { balance, defaultWallet, requestState } = useSelector(
    (state: RootState) => state.wallet
  );
  const dispatch = useDispatch();

  useEffect(() => {
    dispatch(fetchWalletBalance(""));
  }, [dispatch]);


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
      <Button
        onClick={() => {
          dispatch(fetchWalletBalance(""));
        }}
      >
        Get Balance
      </Button>
      <Title order={4}>Balance: {balance}</Title>
      {requestState !== "idle" && <Loader />}
    </Stack>
  );
};
