import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/prompt': 'http://127.0.0.1:8188',
      '/queue': 'http://127.0.0.1:8188',
      '/interrupt': 'http://127.0.0.1:8188',
      '/history': 'http://127.0.0.1:8188',
      '/object_info': 'http://127.0.0.1:8188',
      '/system_stats': 'http://127.0.0.1:8188',
      '/embeddings': 'http://127.0.0.1:8188',
      '/models': 'http://127.0.0.1:8188',
      '/extensions': 'http://127.0.0.1:8188',
      '/view': 'http://127.0.0.1:8188',
      '/view_input': 'http://127.0.0.1:8188',
      '/view_video': 'http://127.0.0.1:8188',
      '/view_audio': 'http://127.0.0.1:8188',
      '/list_images': 'http://127.0.0.1:8188',
      '/upload/image': 'http://127.0.0.1:8188',
      '/upload/input_image': 'http://127.0.0.1:8188',
      '/input_images': 'http://127.0.0.1:8188',
      '/workflow': 'http://127.0.0.1:8188',
      '/workflows': 'http://127.0.0.1:8188',
      '/config': 'http://127.0.0.1:8188',
      '/custom_nodes': 'http://127.0.0.1:8188',
      '/agent': 'http://127.0.0.1:8188',
      '/llm': 'http://127.0.0.1:8188',
      '/model_manager': {
        target: 'http://127.0.0.1:8188',
        configure: (proxy) => {
          proxy.on('proxyReq', (proxyReq) => {
            proxyReq.setHeader('Connection', 'keep-alive');
          });
        },
      },
      '/model_downloads': 'http://127.0.0.1:8188',
      '/ws': {
        target: 'ws://127.0.0.1:8188',
        ws: true,
      },
    },
  },
})
