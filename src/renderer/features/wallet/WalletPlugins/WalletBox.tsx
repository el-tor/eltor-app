import {
  Stack,
  Title,
  Center,
  Select,
  Button,
  Box,
  Loader,
  Text,
  Image,
  Badge,
} from "@mantine/core";

import { ChannelBalanceLine } from "renderer/components/ChannelBalanceLine";
import styles from "./WalletBox.module.css";
import { Circle } from "renderer/components/Circle";

export const WalletBox = ({
  logo,
  onClick,
  isDefault,
}: {
  logo: string;
  onClick: () => void;
  isDefault?: boolean;
}) => {
  return (
    <Box
      w={170}
      h={100}
      m="xs"
      p="sm"
      className={styles.box}
      onClick={onClick}
      bg="white"
      style={{
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        alignItems: "center",
        position: "relative",
      }}
    >
      {isDefault && (
        <>
          <Circle
            color="lightgreen"
            styles={{
              position: "absolute",
              top: 6,
              right: 6,
            }}
          />
          <Badge
            color="green"
            variant="filled"
            size="xs"
            style={{ position: "absolute", top: 6, left: 6 }}
          >
            Default
          </Badge>
        </>
      )}

      <Center style={{ width: "100%", height: "100%" }}>
        <Image bg="white" src={logo} />
      </Center>
    </Box>
  );
};
