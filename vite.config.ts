/// <reference types="vitest" />
import path from "path";
import { PluginOption, UserConfig, defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import eslint from "vite-plugin-eslint";
import { coverageConfigDefaults } from "vitest/config";
import dts from "vite-plugin-dts";

// TODO: remove extra compile step when vscode can run esmodules  https://github.com/microsoft/vscode/issues/130367

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */
function makeConfig(library: "browser" | "node") {
  return defineConfig(({ command, mode }) => {
    const OUT_DIR = library === "browser" ? "dist/chat" : "dist/events";
    const CONFIG: UserConfig = {
      // Build the webpage
      define: {
        "process.env.NODE_ENV": JSON.stringify(mode),
      },
      mode,
      build: {
        emptyOutDir: true,
        outDir: OUT_DIR,
        copyPublicDir: false,
        sourcemap: library === "browser",
        rollupOptions: {
          // TODO: remove when this issue is closed https://github.com/vitejs/vite/issues/15012
          onwarn(warning, defaultHandler) {
            if (warning.code === "SOURCEMAP_ERROR") {
              return;
            }

            defaultHandler(warning);
          },
        },
      },
      plugins: [react()],
      server: {
        proxy: {
          "/v1": process.env.REFACT_LSP_URL ?? "http://127.0.0.1:8001",
        },
      },
      test: {
        retry: 2,
        environment: "jsdom",
        coverage: {
          exclude: coverageConfigDefaults.exclude.concat(
            "**/*.stories.@(js|jsx|mjs|ts|tsx)",
          ),
        },
        setupFiles: ["./src/utils/test-setup.ts"],
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
      ]);

      CONFIG.plugins?.push([
        dts({
          outDir: OUT_DIR,
          rollupTypes: true,
          insertTypesEntry: true,
        }),
      ]);

      CONFIG.build = {
        ...CONFIG.build,
        lib: {
          entry:
            library === "browser"
              ? path.resolve(__dirname, "src/lib/index.ts")
              : path.resolve(__dirname, "src/events/index.ts"),
          name: "RefactChat",
          fileName: "index",
        },
      };
    }

    return CONFIG;
  });
}

export default makeConfig("browser");

export const nodeConfig = makeConfig("node");
