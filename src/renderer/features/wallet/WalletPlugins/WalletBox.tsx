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

  import { ChannelBalanceLine } from "renderer/components/ChannelBalanceLine";
  import styles from './WalletBox.module.css';

  
  export const WalletBox = ({logo, onClick}) => {
    return (

        <Box
          w={170}
          h={100}
          m="xs"
          className={styles.box}
          onClick={onClick}
        
        >
          <img
            src={logo}
            style={{
              width: "100%",
              height: "auto",
              filter: "invert(1)",
            }}
          />
        </Box>

    );
  };
  