import { Box } from "@mantine/core";

export function Circle({
  color,
  width = "10px",
  height = "10px",
  borderRadius = "50%",
  styles = {},
}: {
  color: string;
  width?: string;
  height?: string;
  borderRadius?: string;
  styles?: React.CSSProperties;
}) {
  return (
    <Box
      style={{
        width,
        height,
        backgroundColor: color,
        borderRadius,
        ...styles,
      }}
    />
  );
}
