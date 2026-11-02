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

import styles from "./WalletBox.module.css";
import { Circle } from "../../../components/Circle";

export const WalletBox = ({
  logo,
  onClick,
  isDefault,
  ready,
}: {
  logo: string;
  onClick: () => void;
  isDefault?: boolean;
  ready?: boolean;
}) => {
  return (
    <Box
      w={160}
      h={52}
      m="xs"
      p="sm"
      className={styles.box}
      onClick={onClick}
      title={isDefault ? (ready ? "Selected wallet: connected" : "Selected wallet: unavailable") : undefined}
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
            color={ready ? "lightgreen" : "orange"}
            styles={{
              position: "absolute",
              top: 6,
              right: 6,
            }}
          />
          {/* <Badge
            color="green"
            variant="filled"
            size="11"
            style={{ position: "absolute", bottom: -7, center: 0}}
          >
            Default
          </Badge> */}
        </>
      )}

      <Center style={{ width: "70%", height: "70%" }}>
        <Image bg="white" src={logo} />
      </Center>
    </Box>
  );
};
