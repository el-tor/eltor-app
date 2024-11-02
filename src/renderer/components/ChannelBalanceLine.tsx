import { Box, Group, Title, Text, useMantineTheme } from "@mantine/core";

export function ChannelBalanceLine({
  send,
  receive,
}: {
  send: number;
  receive: number;
}) {
  const sendPercentage = (send / (send + receive)) * 100;
  const theme = useMantineTheme();

  return (
    <Box>
      <Box
        style={{
          width: "100%",
          height: "22px",
          background: `linear-gradient(90deg, ${theme.colors.grape[6]} ${sendPercentage}%, ${theme.colors.teal[1]} 0%`,
        }}
      ></Box>
      <Group justify="space-between" mt="5">
        <Title order={6}>
          Can Send: <span style={{ fontFamily: "monospace" }}>{send}</span> sats
        </Title>
        <Title order={6}>
          Can Receive:{" "}
          <span style={{ fontFamily: "monospace" }}>{receive}</span> sats
        </Title>
      </Group>
    </Box>
  );
}
