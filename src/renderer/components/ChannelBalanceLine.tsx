import { Box, Group, Title, Text } from "@mantine/core";

export function ChannelBalanceLine({
  send,
  receive,
}: {
  send: number;
  receive: number;
}) {
  const sendPercentage = (send / (send + receive)) * 100;

  return (
    <Box>
      <Box
        style={{
          width: "100%",
          height: "22px",
          background: `linear-gradient(90deg, purple ${sendPercentage}%, pink 25%`,
        }}
      ></Box>
      <Group justify="space-between" mt="5">
        <Title order={6}>
          Can Send: <span style={{ fontFamily: "monospace" }}>{send}</span>{" "}
          sats
        </Title>
        <Title order={6}>Can Receive: <span style={{ fontFamily: "monospace" }}>{receive}</span> sats</Title>
      </Group>
    </Box>
  );
}
