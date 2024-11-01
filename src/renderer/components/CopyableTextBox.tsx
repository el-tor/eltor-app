import React from "react";
import { Box, Text, Button } from "@mantine/core";
import { useClipboard } from "@mantine/hooks";
import styles from "../globals.module.css";


interface CopyableTextBoxProps {
  text: string;
  maxWidth?: string;
}

const CopyableTextBox: React.FC<CopyableTextBoxProps> = ({
  text,
  maxWidth = styles.maxWidth,
}) => {
  const clipboard = useClipboard({ timeout: 500 });

  return (
    <Box style={{ maxWidth, position: "relative", padding: 6, borderRadius: 4 }} bg="white">
      <Text
        style={{
          overflowWrap: "break-word",
          whiteSpace: "normal",
          color: "black",
          marginTop: 6,
          marginBottom: 6,
          marginLeft: 6,
          marginRight: 6,
          fontFamily: 'monospace',
        }}
      >
        {text}
      </Text>
      <Button
        size="xs"
        style={{ position: "absolute", bottom: 4, right: 4, height: 24 }}
        onClick={() => clipboard.copy(text)}
      >
        {clipboard.copied ? "Copied" : "Copy"}
      </Button>
    </Box>
  );
};

export default CopyableTextBox;
