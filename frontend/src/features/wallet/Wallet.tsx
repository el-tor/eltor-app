import {
  Stack,
  Title,
  Center,
  Box,
  Loader,
  Group,
  SimpleGrid,
} from "@mantine/core";
import { useDispatch, useSelector } from "../../hooks";
import { useEffect, useState } from "react";
import {
  fetchChannelInfo,
  fetchWalletBalance,
  setDefaultWallet,
  getBolt12Offer,
} from "./walletStore";
import { ChannelBalanceLine } from "../../components/ChannelBalanceLine";
import { WalletPlugins } from "./WalletPlugins/WalletPlugins";
import CopyableTextBox from "../../components/CopyableTextBox";
import QRCode from "react-qr-code";
import { IconRefresh } from "@tabler/icons-react";
import { Circle } from "../../components/Circle";
import { Transactions } from "./Transactions";

export interface IWallet {
  getWalletTransactions: (walletId: string) => Promise<any>;
  payInvoice: (invoice: string) => Promise<string>;
  getBolt12Offer: () => Promise<string>;
  fetchWalletBalance: () => Promise<FetchWalletBalanceResponseType>;
  decodeInvoice: (invoice: string) => Promise<any>;
  checkPaymentStatus: (paymentId: string) => Promise<any>;
  fetchChannelInfo: (channelId: string) => Promise<FetchChannelInfoResponseType>;
  onPaymentReceived: (event: any) => void;
}

export type {
  FetchWalletBalanceResponseType,
  WalletProviderType,
  FetchChannelInfoResponseType
}


type FetchWalletBalanceResponseType = {
  balance: number;
};

type FetchChannelInfoResponseType = {
  send: number;
  receive: number;
};

type WalletProviderType = "Phoenixd" | "Lndk" | "CoreLightning" | "Strike" | "None";

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
    <Box>
      {showWallet && (
        <Box w="100%">
          <Group>
            <Center>
              {/* <WalletPlugins
                setShowWallet={setShowWallet}
                showWallet={showWallet}
              /> */}
              <WalletPlugins />
            </Center>
            <Group ml="auto">
              <Center>
                {" "}
                {loading && (
                  <Loader
                    size="sm"
                    style={{
                      visibility: loading ? "visible" : "hidden",
                    }}
                  />
                )}
              </Center>
            </Group>
            <Circle color={defaultWallet ? "lightgreen" : "#FF6347"} />
            {/* <Title order={2}>{defaultWallet}</Title> */}
          </Group>

          <Group mt="xl">
            <Title order={4}>
              Balance:{" "}
              <span style={{ fontFamily: "monospace" }}>{balance}</span>
            </Title>
            <IconRefresh
              stroke={1.5}
              onClick={() => {
                dispatch(fetchWalletBalance(""));
              }}
              style={{ cursor: "pointer" }}
            />
          </Group>
          <ChannelBalanceLine
            send={balance ?? 0}
            receive={channelInfo.receive ?? 0}
          />

          <SimpleGrid
            mt="lg"
            cols={{ base: 1, sm: 2 }} // Stack vertically on small screens, two columns on larger screens
            spacing={{ base: "md", sm: "lg" }} // Adjust spacing based on screen size
            verticalSpacing={{ base: "md", sm: "lg" }} // Adjust vertical spacing based on screen size
          ></SimpleGrid>

          <Box w="100%">
            <Center>
              <Stack bg="white" p="md" style={{ borderRadius: "6px" }}>
                <Center>
                  <Title order={5} mb="xs" style={{ color: "black" }}>
                    BOLT 12 Offer
                  </Title>
                </Center>
                <QRCode
                  value={bolt12Offer}
                  size={280}
                  style={{ border: 2, borderColor: "whitesmoke" }}
                />
                <CopyableTextBox
                  text={bolt12Offer}
                  limitChars={22}
                  bg="white"
                />
              </Stack>
            </Center>
          </Box>

          <Box w="100%" mt="lg">
            <Transactions h="450px" />
          </Box>

          {/* <Button
            w="100%"
            style={{
                visibility: loading ? "hidden" : "visible",
              }}
            onClick={() => {
              dispatch(fetchWalletBalance(""));
            }}
          >
            Get Balance
          </Button> */}
          {/* <Select
              label="Change your default wallet"
              placeholder=""
              value={defaultWallet}
              onChange={(value) => {
                dispatch(setDefaultWallet(value as WalletProviderType));
              }}
              data={["Phoenixd", "Lndk", "CoreLightning", "None"]}
            /> */}
        </Box>
      )}
    </Box>
  );
};
