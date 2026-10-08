import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
export default defineConfig({
  plugins: [vue()],
  server: {
    port: 1427,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**', '**/.deps/**', '**/output/**', '**/artifacts/**'] },
  },
  clearScreen: false,
});
