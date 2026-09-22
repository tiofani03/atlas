import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 31420,
    strictPort: false,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:31415',
        changeOrigin: true,
      },
    },
  },
  build: {
    chunkSizeWarningLimit: 800,
    rollupOptions: {
      output: {
        manualChunks: {
          'xyflow-vendor': ['@xyflow/react', '@dagrejs/dagre'],
        },
      },
    },
  },
});
