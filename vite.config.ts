/// <reference types="vitest" />
import { PluginOption, defineConfig } from 'vite';
import react from '@vitejs/plugin-react-swc';
import eslint from 'vite-plugin-eslint';

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */
export default defineConfig(({command}) => {
  const plugins: PluginOption[] = [react()]
  if(command !== "serve") {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-call
    plugins.push(eslint() as PluginOption)
  }
  return {
    plugins,
    test: {
      environment: "jsdom"
    },
    css: {
      modules: true
    }
  }
})
