import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'path'

export default defineConfig({
  base: process.env.VITE_BASE_PATH || '/',
  plugins: [
    vue(),
    vuetify({ autoImport: true }),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:2241',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:2241',
        ws: true,
      },
      '/stream': {
        target: 'http://localhost:2241',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: '../static',
    emptyOutDir: true,
    chunkSizeWarningLimit: 1500,
  },
})
