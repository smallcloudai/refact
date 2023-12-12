/// <reference types="vitest" />
import { PluginOption, defineConfig } from 'vite';
import react from '@vitejs/plugin-react-swc';
import { eslint } from 'vite-plugin-eslint';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    // eslint-disable-next-line @typescript-eslint/no-unsafe-call
    eslint(),
  ] as PluginOption[],
  test: {
    environment: "jsdom",
  }
})
