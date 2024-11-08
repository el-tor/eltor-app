import { createTheme, MantineProvider, rem } from "@mantine/core";

export const theme = createTheme({
  colors: {
    // Add your color
    purple: [
      "#f5e6ff", // Very light lavender
      "#e5d1fa", // Light lavender
      "#d4baf5", // Soft lilac
      "#c3a1f0", // Pale purple
      "#b388eb", // Light purple
      "#a273e8", // Medium light purple
      "#9163d7", // Purple
      "#8050c6", // Darker purple
      "#7044b1", // Deep purple
      "#60379c", // Very deep purple
    ],
    teal: [
      "#E6FFFA", // Very light teal, almost like a pastel turquoise
      "#B2F5EA", // Light green
      "#80E2D0", // Soft teal
      "#4DC0B5", // Turquoise
      "#38A196", // Medium teal
      "#2A847C", // Deep teal
      "#226F68", // Darker teal for contrast
      "#1B5E55", // Dark teal, almost like forest green but with blue tones
      "#144D44", // Very dark teal, good for text or dark mode elements
      "#0F3D37", // Deepest teal, almost black with a hint of green-blue
    ],
    deepBlue: [
      "#eef3ff",
      "#dce4f5",
      "#b9c7e2",
      "#94a8d0",
      "#748dc1",
      "#5f7cb8",
      "#5474b4",
      "#44639f",
      "#39588f",
      "#2d4b81",
    ],
    // or replace default theme color
    blue: [
      "#eef3ff",
      "#dee2f2",
      "#bdc2de",
      "#98a0ca",
      "#7a84ba",
      "#6672b0",
      "#5c68ac",
      "#4c5897",
      "#424e88",
      "#364379",
    ],
    black: [
      "#1e1e1e",
      "#f5f5f5",
      "#e5e5e5",
      "#d4d4d4",
      "#c3c3c3",
      "#b3b3b3",
      "#a2a2a2",
      "#919191",
      "#818181",
      "#707070",
      "#606060",
    ]
  },

  primaryColor: "purple",
  // primaryShade: 5,

  shadows: {
    md: "1px 1px 3px rgba(0, 0, 0, .25)",
    xl: "5px 5px 3px rgba(0, 0, 0, .25)",
  },

  headings: {
    fontFamily: "Roboto, sans-serif",
    sizes: {
      h1: { fontSize: rem(36) },
    },
  },
});
