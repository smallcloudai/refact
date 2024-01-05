/// <reference types="vitest" />
import path from "path";
import { PluginOption, UserConfig, defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import eslint from "vite-plugin-eslint";
import { coverageConfigDefaults } from "vitest/config";

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */

const DEFAULT_CONFIG = defineConfig(({ command, mode }) => {
  const plugins: PluginOption[] = [react()];
  if (command !== "serve") {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-call
    plugins.push(eslint() as PluginOption);
  }

  return {
    // Build the webpage
    define: {
      "process.env.NODE_ENV": JSON.stringify(mode),
    },
    build: {
      outDir: "dist/webpage",
    },
    plugins,

    server: {
      proxy: {
        "/v1": process.env.REFACT_LSP_URL ?? "http://127.0.0.1:8001",
      },
    },
    test: {
      environment: "jsdom",
      coverage: {
        exclude: coverageConfigDefaults.exclude.concat(
          "**/*.stories.@(js|jsx|mjs|ts|tsx)",
        ),
      },
    },
    css: {
      modules: {},
    },
  };
});

export default DEFAULT_CONFIG;

export const LIB_CONFIG: UserConfig = {
  ...DEFAULT_CONFIG,
  mode: "production",
  define: { "process.env.NODE_ENV": "'production'" },
  build: {
    outDir: "dist/lib",
    lib: {
      entry: path.resolve(__dirname, "src/self-hosted-chat.tsx"),
      name: "RefactChat",
      fileName: "chat",
    },
    // rollupOptions: {
    //   external: ["react/jsx-runtime"],
    // },
  },
};
