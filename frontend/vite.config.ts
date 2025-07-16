import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],

  // Tauri expects a relative base path
  base: './',

  build: {
    // Don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // Produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
    outDir: 'dist',
    emptyOutDir: true,
  },
  // Prevent dev server issues
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    open: true,
    // Proxy for web mode
    // Only run proxy in development mode (not in Tauri)
    ...(process.env.NODE_ENV === 'development' ? {
      proxy: {
        '/api': {
          target: 'http://localhost:5174',
          changeOrigin: true,
        },
        '/health': {
          target: 'http://localhost:5174',
          changeOrigin: true,
        }
      }
    } : {}),
  },
  define: {
    global: 'globalThis',
  },
  envPrefix: ['VITE_', 'TAURI_'],
})