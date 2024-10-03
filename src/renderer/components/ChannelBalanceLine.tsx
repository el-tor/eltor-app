import { Box, Group, Title } from "@mantine/core";

export function ChannelBalanceLine({
  send,
  receive,
}: {
  send: number;
  receive: number;
}) {
  const sendPercentage = (send / (send + receive)) * 100;

  return (
    <>
      <Box
        style={{
          width: "100%",
          height: "22px",
          background: `linear-gradient(90deg, purple ${sendPercentage}%, pink 25%`,
        }}
      ></Box>
      <Group justify="space-between">
        <Title order={5}>Can Send: {send} sats</Title>
        <Title order={5}>Can Receive: {receive} sats</Title>
      </Group>
    </>
  );
}
