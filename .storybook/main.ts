import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  stories: ["../src/**/*.mdx", "../src/**/*.stories.@(js|jsx|mjs|ts|tsx)"],
  addons: [
    "@storybook/addon-links",
    "@storybook/addon-essentials",
    "@storybook/addon-onboarding",
    "@storybook/addon-interactions",
  ],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
  docs: {
    autodocs: "tag",
  },
  viteFinal: (config, options) => {
    const server = {
      ...config.server,
      proxy: {
        "/v1": process.env.REFACT_LSP_URL ?? "http://127.0.0.1:8001",
      },
    };

    return { ...config, server };
  },
  staticDirs: ["../public", "../dist"],
};
export default config;
