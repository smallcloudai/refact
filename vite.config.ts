/// <reference types="vitest" />
import path from "path";
import { PluginOption, defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import eslint from "vite-plugin-eslint";
import { coverageConfigDefaults } from "vitest/config";

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */
// can return an array for multiple builds

const LIB_OPTIONS = {
  entry: path.resolve(__dirname, "src/self-hosted-chat.tsx"),
  name: "RefactChat",
  fileName: "chat",
};

const DEFAULT_OPTIONS = {
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

export default defineConfig(({ command, mode }) => {
  const plugins: PluginOption[] = [react()];
  if (command !== "serve") {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-call
    plugins.push(eslint() as PluginOption);
  }

  const WEBPAGE_CONFIG = {
    // Build the webpage
    build: {
      outDir: "dist/webpage",
    },
    plugins,
    ...DEFAULT_OPTIONS,
  };

  const LIB_CONFIG = {
    // Build the library
    build: {
      outDir: "dist/lib",
      lib: LIB_OPTIONS,
    },
    plugins,
    ...DEFAULT_OPTIONS,
  };

  if (mode === "library") {
    return LIB_CONFIG;
  }

  return WEBPAGE_CONFIG;
});
