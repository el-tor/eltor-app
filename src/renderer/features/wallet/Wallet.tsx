import {
  Stack,
  Title,
  Center,
  Box,
  Loader,
  Group,
  Grid,
  Select,
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
import CopyableTextBox from "renderer/components/CopyableTextBox";
import QRCode from "react-qr-code";
import { IconRefresh } from "@tabler/icons-react";
import { Circle } from "renderer/components/Circle";

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
          <Group w="100%">
            <Circle color={defaultWallet ? "lightgreen" : "#FF6347"} />
            <Title order={2}>{defaultWallet}</Title>
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
          </Group>

          <Group mt="md">
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

          <Center mt="lg">
            <Box
              bg="white"
              p="sm"
              m="lg"
              style={{ borderRadius: "6px", width: "300px" }}
            >
              <Center>
                <Title order={5} mb="xs" style={{ color: "black" }}>
                  BOLT 12 Offer
                </Title>
              </Center>
              <QRCode value={bolt12Offer} size={280} style={{border:2, borderColor:"whitesmoke"}} />
              <CopyableTextBox text={bolt12Offer} limitChars={22} bg="white" />
            </Box>
          </Center>

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
              data={["Phoenixd", "Lndk", "CoreLightning", "None"]}
            /> */}
          </Stack>
        </Box>
      )}
    </Stack>
  );
};
