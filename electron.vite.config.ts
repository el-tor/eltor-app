import { defineConfig, externalizeDepsPlugin } from 'electron-vite'
import tsconfigPathsPlugin from 'vite-tsconfig-paths'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

import { settings } from './src/lib/electron-router-dom'

const tsconfigPaths = tsconfigPathsPlugin({
  projects: [resolve('tsconfig.json')],
})

export default defineConfig({
  main: {
    plugins: [tsconfigPaths, externalizeDepsPlugin()],
  },

  preload: {
    plugins: [tsconfigPaths, externalizeDepsPlugin()],
  },

  renderer: {
    plugins: [tsconfigPaths, react()],

    css: {
      postcss: {
        plugins: [],
      },
    },

    server: {
      port: settings.port,
    },
  },
})
