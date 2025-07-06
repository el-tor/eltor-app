import type React from 'react';
import { MantineProvider, createTheme } from '@mantine/core';
import { theme } from './theme';

export function MantineWrapper({ children }: { children: React.ReactNode }) {
  return (
    <MantineProvider theme={theme} defaultColorScheme="dark">
      {children}
    </MantineProvider>
  );
}