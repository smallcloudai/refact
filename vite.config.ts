/// <reference types="vitest" />
import path from "path";
import { PluginOption, UserConfig, defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import eslint from "vite-plugin-eslint";
import { coverageConfigDefaults } from "vitest/config";
import dts from "vite-plugin-dts";

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */
export default defineConfig(({ command, mode }) => {
  const CONFIG: UserConfig = {
    // Build the webpage
    define: {
      "process.env.NODE_ENV": JSON.stringify(mode),
    },
    mode,
    build: {
      emptyOutDir: true,
      outDir: "dist",
    },
    plugins: [react()],
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

  if (command !== "serve") {
    CONFIG.mode = "production";
    CONFIG.define = { "process.env.NODE_ENV": "'production'" };
    CONFIG.plugins?.push([
      // eslint-disable-next-line @typescript-eslint/no-unsafe-call
      eslint() as PluginOption,
      dts({ rollupTypes: true }),
    ]);
    CONFIG.build = {
      outDir: "dist",
      lib: {
        // TODO: make entry an  object
        entry: path.resolve(__dirname, "src/lib/index.ts"),
        name: "RefactChat",
        fileName: "chat",
      },
    };
  }

  return CONFIG;
});
