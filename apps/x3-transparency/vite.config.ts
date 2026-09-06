import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  base: '/transparency/',
  server: { port: 1450, strictPort: true },
})
