import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      // The frontend always calls the versioned, absolute `/api/v1/...`
      // path (see src/api/client.ts) so this proxy is the *only* place
      // that knows the backend's dev address. In production the built
      // SPA is served by the same Axum process that owns `/api/v1`, so
      // requests are same-origin and this block is dev-only.
      '/api/v1': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
})
