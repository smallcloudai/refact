/// <reference types="vitest" />
import { PluginOption, defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import eslint from "vite-plugin-eslint";
import { coverageConfigDefaults } from "vitest/config";

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */
export default defineConfig(({ command }) => {
  const plugins: PluginOption[] = [react()];
  if (command !== "serve") {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-call
    plugins.push(eslint() as PluginOption);
  }
  return {
    server: {
      proxy: {
        // TODO: make this an env var
        // https://vitejs.dev/config/#using-environment-variables-in-config
        "/v1": "http://localhost:8001",
      },
    },
    plugins,
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
